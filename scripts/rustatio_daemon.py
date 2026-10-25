#!/usr/bin/env python3

import os
import sys
import time
import json
import logging
from logging.handlers import RotatingFileHandler
import functools
import threading
import random
import re
import shutil
import urllib.parse
import requests
import copy
import datetime
import ast
import operator
import hmac
from collections import defaultdict
from flask import Flask, request, jsonify, render_template_string, make_response, redirect

app = Flask(__name__)

# ==========================================
# CONFIGURATION
# ==========================================
DEFAULT_CONFIG = {
    'PORT': 8080,                                 # Rustatio's port
    'RUSTATIO_API': "http://127.0.0.1",           # Rustatio's ip
    'ADMIN_PORT': "",                             # Daemon admin port (blank to use Rustatio)
    'AUTH_TOKEN': "",                             # If you want another token for admin panel (blank to use Rustatio)
    'REFRESH_INTERVAL': 0,                        # 0 to use scrape interval from Rustatio (hardcoded 10s min to avoid self spam)
    'ARCHIVE_FOLDER': "/data/archived",           # (blank to disable)
    'RULES_FILE': "/data/rules.txt",              # Important !
    'DEFAULTS_FILE': "/data/state.json",          # Important !
    'DRY_RUN': False,                             # Maybe i forgot to push this everywhere
    'LOGFILE': "/data/rustatio_daemon.log",       # Important !
    'CHECK_LOGS_FILE': "/data/check_logs.json",   # Important !
    'LOGS_WATCHER': 1,                            # 0/1 Watch Rustatio's log for errors and pause a torrent !!!Set log level in Rustatio to INFO!!!
    'RUST_DAEMON_LOG': "",                        # If you want another log level in admin panel only
    'WATCHER_MAX_STRIKE': 3,                      # 0-50 torrents -> 3, increase by 2 for 100+ torrent to avoid false positives
    'WATCHER_STRIKE_TIME': 3600,                  # 30min (1800s) is close
    'WATCHER_PAUSE_TIME': 3600,                   # How much time you will be flagged as a spammer ? -> 3600s (1h) for most trackers
    'WATCHER_ANNOUNCE_TIMEOUT': 130,              # Used to filter (tcp timeout) warns and errors in BUGGED Rustatio's logs (only with Rustatio's logs on INFO)
    'TOR_KEEP_LAST': 1                            # 0/1 keep alive last torrent for a tracker
}

# Config's key locked
READONLY_KEYS = {
    'PORT', 'ADMIN_PORT', 'RUSTATIO_API',
    'ARCHIVE_FOLDER', 'RULES_FILE', 'DEFAULTS_FILE', 
    'LOGFILE', 'CHECK_LOGS_FILE'
}

# Something weird but cool
FAVICON_SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
    <circle cx="32" cy="32" r="30" fill="#121212" stroke="#ce412b" stroke-width="3"/>
    <path d="M32 10 a22 22 0 1 0 0.001 0" fill="none" stroke="#ce412b" stroke-width="4" stroke-dasharray="6,6"/>
    <text x="32" y="43" font-family="sans-serif" font-weight="900" font-size="34" fill="#ce412b" text-anchor="middle">R</text>
</svg>"""

# ==============================================================================
# 2. CONFIG MANAGER
# ==============================================================================
def parse_value(val, sample_value):
    if isinstance(sample_value, bool):
        return str(val).lower() in ("true", "1", "yes")
    if isinstance(sample_value, int):
        if str(val).lower() in ("true", "yes"):
            return 1
        if str(val).lower() in ("false", "no"):
            return 0
        return int(val)
    return str(val)

class ConfigManager:
    def __init__(self, defaults: dict, readonly_keys: set):
        self.readonly_keys = readonly_keys
        self.initial_state = {}
        self.state = {}
        self._load(defaults)

    def _sync_to_globals(self): # globals part
        globals().update(self.state) # globals part

    def _load(self, defaults: dict):
        for key, default_val in defaults.items():
            env_val = os.getenv(key)
            self.initial_state[key] = parse_value(env_val, default_val) if env_val is not None else default_val

        port = self.initial_state['PORT']
        admin_port = self.initial_state['ADMIN_PORT']

        self.initial_state['ADMIN_PORT'] = int(admin_port) if admin_port else port + 1
        self.initial_state['RUSTATIO_API'] = os.getenv("RUSTATIO_API", f"http://127.0.0.1:{port}")

        self.state = self.initial_state.copy()
        self._sync_to_globals() # globals part

    def update(self, new_data: dict):
        forbidden = [k for k in new_data if k in self.readonly_keys]
        if forbidden:
            return False, f"Modification interdite pour : {', '.join(forbidden)}"

        ALLOWED_LOG_LEVELS = {"", "TRACE", "DEBUG", "INFO", "WARN", "ERROR"}

        for key, val in new_data.items():
            if key in self.state:
                if key == 'RUST_DAEMON_LOG':
                    val_str = str(val).strip().upper()
                    if val_str not in ALLOWED_LOG_LEVELS:
                        return False, f"Valeur invalide pour RUST_DAEMON_LOG. Autorisé : {', '.join(sorted(ALLOWED_LOG_LEVELS))}"
                    val = val_str

                sample_val = self.initial_state.get(key, "")
                parsed_val = parse_value(val, sample_val)
                self.state[key] = parsed_val
                
                os.environ[key] = str(parsed_val)
                
                if key == 'RUST_DAEMON_LOG':
                    new_level = RUST_LOG_LEVELS.get(val_str, logging.INFO)
                    logging.getLogger("Rustatio").setLevel(new_level)

        self._sync_to_globals() # globals part
        return True, "Variables mises à jour temporairement"

    def reset(self):
        for key, value in self.initial_state.items():
            if key not in self.readonly_keys:
                self.state[key] = value
                os.environ[key] = str(value)

                if key == 'RUST_DAEMON_LOG':
                    val_str = str(value).strip().upper()
                    new_level = RUST_LOG_LEVELS.get(val_str, logging.INFO)
                    logging.getLogger("Rustatio").setLevel(new_level)

        self._sync_to_globals() # globals part

config = ConfigManager(DEFAULT_CONFIG, READONLY_KEYS)

# ==========================================
# FOLDERS AND FILES INITIALISATION
# ==========================================
def ensure_paths_exist():
    if RULES_FILE:
        rules_dir = os.path.dirname(RULES_FILE)
        if rules_dir:
            os.makedirs(rules_dir, exist_ok=True)
        if not os.path.exists(RULES_FILE):
            with open(RULES_FILE, "w", encoding="utf-8") as f:
                f.write("# Fichier de règles Rustatio\n")

    if DEFAULTS_FILE:
        defaults_dir = os.path.dirname(DEFAULTS_FILE)
        if defaults_dir:
            os.makedirs(defaults_dir, exist_ok=True)
        if not os.path.exists(DEFAULTS_FILE):
            with open(DEFAULTS_FILE, "w", encoding="utf-8") as f:
                json.dump({"default_config": {}}, f, indent=4)

# ==========================================
# HTML TEMPLATE (ADMIN PANEL)
# ==========================================
LOGIN_TEMPLATE = """
<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rustatio - Authentification</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background-color: #121212;
            color: #e0e0e0;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
        }
        .login-card {
            background: #1e1e1e;
            border: 1px solid #3d3d3d;
            border-radius: 8px;
            padding: 30px;
            width: 100%;
            max-width: 360px;
            box-shadow: 0 4px 15px rgba(0, 0, 0, 0.5);
            text-align: center;
        }
        h2 { color: #ce412b; margin-top: 0; margin-bottom: 20px; }
        input[type="password"] {
            width: 100%;
            padding: 10px;
            background: #0d0d0d;
            color: #fff;
            border: 1px solid #3d3d3d;
            border-radius: 4px;
            box-sizing: border-box;
            margin-bottom: 15px;
            font-size: 1em;
        }
        input[type="password"]:focus { outline: 1px solid #ce412b; }
        button {
            width: 100%;
            background-color: #ce412b;
            color: white;
            border: none;
            padding: 10px;
            border-radius: 4px;
            font-weight: bold;
            cursor: pointer;
            font-size: 1em;
            transition: background 0.2s;
        }
        button:hover { background-color: #e84d35; }
        .error { color: #f44336; font-size: 0.85em; margin-bottom: 10px; display: none; }
    </style>
</head>
<body>
    <div class="login-card">
        <div style="text-align: right; margin-bottom: 10px;">
            <span onclick="setLang('fr')" style="cursor: pointer; margin-right: 5px;" id="btn-fr">🇫🇷</span>
            <span onclick="setLang('en')" style="cursor: pointer;" id="btn-en">🇬🇧</span>
        </div>
        <h2 id="i-title">🔒 Connexion Rustatio</h2>
        {% if error %}
            <div class="error" style="display:block;" id="i-err">Token invalide, veuillez réessayer.</div>
        {% endif %}
        <form method="POST" action="/login">
            <input type="password" name="token" id="i-input" placeholder="Entrez votre jeton d'accès..." required autofocus>
            <button type="submit" id="i-btn">Se connecter</button>
        </form>
    </div>
    <script>
        const i18nLogin = {
            fr: { title: "🔒 Connexion Rustatio", err: "Token invalide, veuillez réessayer.", ph: "Entrez votre jeton d'accès...", btn: "Se connecter" },
            en: { title: "🔒 Rustatio Login", err: "Invalid token, please try again.", ph: "Enter your access token...", btn: "Login" }
        };
        let currentLang = localStorage.getItem('rustatio_lang') || 'fr';
        function setLang(lang) {
            currentLang = lang;
            localStorage.setItem('rustatio_lang', lang);
            document.getElementById('i-title').innerText = i18nLogin[lang].title;
            if (document.getElementById('i-err')) document.getElementById('i-err').innerText = i18nLogin[lang].err;
            document.getElementById('i-input').placeholder = i18nLogin[lang].ph;
            document.getElementById('i-btn').innerText = i18nLogin[lang].btn;
            
            document.getElementById('btn-fr').style.opacity = lang === 'fr' ? '1' : '0.5';
            document.getElementById('btn-en').style.opacity = lang === 'en' ? '1' : '0.5';
        }
        window.onload = () => setLang(currentLang);
    </script>
</body>
</html>
"""

HTML_TEMPLATE = """
<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="UTF-8">
    <title>Rustatio - Panneau de Contrôle</title>
    <style>
        :root {
            --bg-color: #121212;
            --panel-bg: #1e1e1e;
            --section-bg: #262626;
            --text-main: #e0e0e0;
            --rust-orange: #ce412b;
            --rust-orange-hover: #e84d35;
            --rust-orange-glow: rgba(232, 77, 53, 0.85);
            --glow-shadow: 0 0 0 2px var(--rust-orange), 0 0 14px 4px rgba(206, 65, 43, 0.75);
            --border-color: #3d3d3d;
            --border-hover: #666666;
            --input-bg: #0d0d0d;
            --input-bg-hover: #161616;
            --success: #4caf50;
            --success-hover: #66bb6a;
            --danger: #f44336;
            --danger-hover: #ef5350;
        }
        
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background-color: var(--bg-color);
            color: var(--text-main);
            margin: 0;
            padding: 20px;
            display: flex;
            flex-direction: column;
            align-items: center;
        }
        
        .container {
            width: 100%;
            max-width: 1450px;
            display: flex;
            flex-direction: column;
            gap: 20px;
        }
        
        .header {
            width: 100%;
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 15px 20px;
            background: var(--panel-bg);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            box-sizing: border-box;
        }
        
        h1, h2 { margin: 0; color: var(--rust-orange); }
        
        .panel {
            background: var(--panel-bg);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 20px;
            display: flex;
            flex-direction: column;
        }
        
        .controls { display: flex; gap: 10px; align-items: center; }
        
        button {
            background-color: var(--rust-orange);
            color: white;
            border: 1px solid transparent;
            padding: 10px 15px;
            border-radius: 4px;
            cursor: pointer;
            font-weight: bold;
            outline: none;
            transition: all 0.2s ease-in-out;
        }
        
        button:hover {
            background-color: var(--rust-orange-hover);
            transform: translateY(-1px);
            box-shadow: 0 0 16px 3px rgba(232, 77, 53, 0.65);
        }
        
        button:active {
            transform: translateY(0);
            box-shadow: 0 0 8px 1px rgba(232, 77, 53, 0.5);
        }
        
        button:focus-visible {
            box-shadow: var(--glow-shadow);
            border-color: #fff;
        }
        
        button.stop { background-color: var(--danger); }
        button.stop:hover {
            background-color: var(--danger-hover);
            box-shadow: 0 0 16px 3px rgba(244, 67, 54, 0.65);
        }
        
        button.start { background-color: var(--success); }
        button.start:hover {
            background-color: var(--success-hover);
            box-shadow: 0 0 16px 3px rgba(76, 175, 80, 0.65);
        }
        
        button.small { padding: 4px 8px; font-size: 0.8em; }
        
        button:disabled {
            opacity: 0.4;
            cursor: not-allowed;
            filter: grayscale(100%);
            pointer-events: none;
            box-shadow: none;
            transform: none;
        }
        
        select, input[type="text"], input[type="number"] {
            background: var(--input-bg);
            color: #fff;
            border: 1px solid var(--border-color);
            padding: 7px 10px;
            border-radius: 4px;
            font-size: 0.9em;
            outline: none;
            transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease;
        }
        
        select:hover, input[type="text"]:hover, input[type="number"]:hover {
            border-color: var(--border-hover);
            background-color: var(--input-bg-hover);
            box-shadow: 0 0 6px rgba(255, 255, 255, 0.1);
        }
        
        select:focus, input[type="text"]:focus, input[type="number"]:focus {
            border-color: var(--rust-orange-hover);
            background-color: var(--input-bg-hover);
            box-shadow: var(--glow-shadow);
        }
        
        .cond-logop {
            font-weight: bold;
            color: var(--rust-orange);
            border-color: var(--rust-orange);
        }
        
        input[type="text"], input[type="number"] { flex-grow: 1; min-width: 100px; }
        
        .rule-row {
            display: flex;
            gap: 12px;
            align-items: stretch;
            background: #181818;
            padding: 12px;
            border-radius: 8px;
            margin-bottom: 15px;
            border: 1px solid var(--border-color);
            position: relative;
        }
        
        .rule-block {
            background: var(--section-bg);
            border: 1px solid var(--border-color);
            border-radius: 6px;
            padding: 10px;
            display: flex;
            flex-direction: column;
            gap: 8px;
        }
        
        .rule-block-title {
            font-size: 0.75em;
            font-weight: bold;
            letter-spacing: 1px;
            color: #888;
            text-transform: uppercase;
            border-bottom: 1px solid #333;
            padding-bottom: 4px;
            margin-bottom: 2px;
        }
        
        .block-conditions { flex: 3; }
        .block-action { flex: 1; min-width: 160px; }
        .block-assign { flex: 2; min-width: 220px; }
        
        .rule-arrow {
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--rust-orange);
            font-size: 1.4em;
            font-weight: bold;
            user-select: none;
        }
        
        .conditions-wrapper {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }
        
        .condition-block {
            display: flex;
            gap: 6px;
            align-items: center;
            background: #1e1e1e;
            padding: 6px;
            border-radius: 4px;
            border: 1px solid #333;
        }
        
        .action-type {
            font-weight: bold;
            text-transform: uppercase;
            padding: 8px;
            border-radius: 4px;
            cursor: pointer;
            border: 1px solid transparent;
            transition: all 0.2s ease;
        }
        
        .action-type:hover {
            filter: brightness(1.3);
            transform: translateY(-1px);
            box-shadow: 0 0 12px rgba(255, 255, 255, 0.2);
        }
        
        .action-type[data-action="start"] { background-color: #1b5e20; color: #a5d6a7; border-color: #2e7d32; }
        .action-type[data-action="stop"] { background-color: #b71c1c; color: #ffcdd2; border-color: #c62828; }
        .action-type[data-action="pause"] { background-color: #e65100; color: #ffe0b2; border-color: #f57c00; }
        .action-type[data-action="resume"] { background-color: #0d47a1; color: #bbdefb; border-color: #1565c0; }
        .action-type[data-action="delete"] { background-color: #4a148c; color: #e1bee7; border-color: #6a1b9a; }
        .action-type[data-action="update"] { background-color: #004d40; color: #b2dfdb; border-color: #00695c; }
        .action-type[data-action="addtags"] { background-color: #006064; color: #b2ebf2; border-color: #00838f; }
        .action-type[data-action="removetags"] { background-color: #4e342e; color: #d7ccc8; border-color: #6d4c41; }
        
        .btn-delete-rule {
            align-self: center;
            background: #333;
            color: #ff6b6b;
            border: 1px solid #555;
            border-radius: 50%;
            width: 28px;
            height: 28px;
            padding: 0;
            display: flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            outline: none;
            transition: all 0.2s ease;
        }
        
        .btn-delete-rule:hover {
            background: var(--danger);
            color: white;
            border-color: var(--danger);
            transform: scale(1.15);
            box-shadow: 0 0 14px 2px rgba(244, 67, 54, 0.75);
        }
        
        .btn-delete-rule:focus-visible {
            box-shadow: var(--glow-shadow);
        }
        
		.raw-editor-container { margin-top: 15px; display: none; }
		
        textarea {
            width: 100%;
            height: 150px;
            background: var(--input-bg);
            color: #fff;
            border: 1px solid var(--border-color);
            padding: 10px;
            font-family: monospace;
            resize: vertical;
            box-sizing: border-box;
            border-radius: 4px;
            outline: none;
            transition: border-color 0.2s ease, box-shadow 0.2s ease;
        }
        
        textarea:focus {
            border-color: var(--rust-orange-hover);
            box-shadow: var(--glow-shadow);
        }
        
        #logs {
            width: 100%;
            height: 400px;
            background: #000;
            color: #a5d6a7;
            border: 1px solid var(--border-color);
            padding: 10px;
            font-family: monospace;
            overflow-y: scroll;
            box-sizing: border-box;
            white-space: pre-wrap;
            border-radius: 4px;
        }
        
        .status-badge {
            padding: 5px 10px;
            border-radius: 12px;
            font-size: 0.9em;
            font-weight: bold;
        }
        
        .status-running { background: rgba(76, 175, 80, 0.2); color: var(--success); }
        .status-stopped { background: rgba(244, 67, 54, 0.2); color: var(--danger); }
        
        .refresh-spinner {
            display: inline-block;
            width: 12px;
            height: 12px;
            border: 2px solid rgba(206, 65, 43, 0.3);
            border-radius: 50%;
            border-top-color: var(--rust-orange);
            animation: spin 0.8s linear infinite;
            opacity: 0;
            transition: opacity 0.2s ease;
            margin-left: 8px;
            vertical-align: middle;
        }
        
        .refresh-spinner.active { opacity: 1; }
        
        .refresh-slot {
            position: relative;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            width: 1cm;
            height: 16px;
            margin: 0 8px;
            flex-shrink: 0;
        }
        
        .progress-container {
            width: 100%;
            height: 4px;
            background-color: #2a2a2a;
            border-radius: 2px;
            overflow: hidden;
            transition: opacity 0.2s ease;
        }
        
        .progress-bar {
            height: 100%;
            width: 0%;
            background-color: var(--rust-orange);
        }
        
        .refresh-slot .refresh-spinner {
            position: absolute;
            opacity: 0;
            transition: opacity 0.2s ease;
            pointer-events: none;
        }
        
        .refresh-slot .refresh-spinner.active { opacity: 1; }
        
        .refresh-slot:has(.refresh-spinner.active) .progress-container { opacity: 0; }
        
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
        
        .collapsible-header {
            cursor: pointer;
            user-select: none;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .collapse-icon {
            display: inline-block;
            transition: transform 0.2s ease;
            margin-right: 8px;
            font-size: 0.8em;
            color: var(--rust-orange);
        }
        
        .panel.collapsed .collapse-content { display: none !important; }
        
        .lang-container {
            display: inline-flex;
            align-items: center;
            margin-right: 15px;
        }
        
        .lang-select {
            background: var(--input-bg);
            color: #fff;
            border: 1px solid var(--border-color);
            border-radius: 6px;
            padding: 6px 12px;
            font-size: 12px;
            font-weight: 600;
            cursor: pointer;
            outline: none;
            transition: all 0.2s ease;
        }
        
        .lang-select:hover {
            background: var(--input-bg-hover);
            border-color: var(--border-hover);
            box-shadow: 0 0 8px rgba(255, 255, 255, 0.1);
        }
        
        .lang-select:focus {
            border-color: var(--rust-orange-hover);
            box-shadow: var(--glow-shadow);
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="panel collapsed" id="panel-control">
            <div class="collapsible-header" onclick="togglePanel('panel-control')">
                <h2 style="display: flex; align-items: center;">
                    <span class="collapse-icon">▶</span> 
                    <span data-i18n="ctrl_title" style="margin-right: 15px;">⚙️ Rustatio Control</span>
                    <div class="lang-container" onclick="event.stopPropagation()">
                        <select id="lang-select" class="lang-select" onchange="setLang(this.value)">
                            <option value="fr">🇫🇷 FR</option>
                            <option value="en">🇬🇧 EN</option>
                        </select>
                        <div style="display:none;">
                            <button id="btn-fr"></button>
                            <button id="btn-en"></button>
                        </div>
                    </div>
                </h2>
                <div class="controls" onclick="event.stopPropagation()">
                    <span id="daemon-status" class="status-badge status-stopped" data-i18n="daemon_check">Vérification...</span>
                    <button id="btn-daemon-start" class="start" onclick="daemonAction('start')" data-i18n="btn_daemon_start">▶ Démarrer Daemon</button>
                    <button id="btn-daemon-stop" class="stop" onclick="daemonAction('stop')" data-i18n="btn_daemon_stop">⏹ Arrêter Daemon</button>
                    <button id="btn-daemon-restart" onclick="daemonAction('restart')" data-i18n="btn_daemon_restart">🔄 Redémarrer Daemon</button>
                    <button style="background-color: #555;" onclick="restartAdmin()" data-i18n="btn_script_restart">♻️ Redémarrer Script</button>
                </div>
            </div>

            <div class="collapse-content" style="margin-top: 15px;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;">
                    <span style="font-size: 0.8em; font-weight: bold; color: #888; text-transform: uppercase; letter-spacing: 1px;" data-i18n="env_title">
                        🔧 Configuration de l'environnement (Modifications temporaires)
                    </span>
                    <div style="display: flex; gap: 8px;">
                        <button onclick="resetEnvConfig()" class="small" style="background: #444;" data-i18n="btn_reset">🔄 Réinitialiser</button>
                        <button onclick="saveEnvConfig()" class="small" style="background: var(--rust-orange);" data-i18n="btn_apply">⚡ Appliquer à la session</button>
                    </div>
                </div>
                <div id="env-config-grid" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 10px;">
                    <div style="color: #666; font-size: 0.85em;" data-i18n="env_loading">Chargement des variables...</div>
                </div>
            </div>
        </div>

        <div class="panel collapsed" id="panel-watcher">
            <div class="collapsible-header" onclick="togglePanel('panel-watcher')">
                <h2 style="display: flex; align-items: center;">
                    <span class="collapse-icon">▶</span> <span data-i18n="watcher_title">👁️️ Log Watcher</span>
                    <div class="refresh-slot">
                        <div class="progress-container">
                            <div id="watcher-progress-bar" class="progress-bar"></div>
                        </div>
                        <span id="watcher-spinner" class="refresh-spinner" title="Mise à jour..."></span>
                    </div>
                </h2>
                <div class="controls" onclick="event.stopPropagation()">
                    <span id="watcher-status" class="status-badge status-stopped" data-i18n="daemon_check">Vérification...</span>
                    <button id="btn-watcher-start" class="start" onclick="watcherAction('start')" data-i18n="btn_start">▶ Démarrer</button>
                    <button id="btn-watcher-stop" class="stop" onclick="watcherAction('stop')" data-i18n="btn_stop">⏹ Arrêter</button>
                    <button id="btn-watcher-restart" onclick="watcherAction('restart')" data-i18n="btn_restart">🔄 Redémarrer</button>
                </div>
            </div>

            <div class="collapse-content" style="margin-top: 15px;">
                <div style="background: #181818; border: 1px solid var(--border-color); border-radius: 8px; padding: 10px; overflow-x: auto;">
                    <table style="width: 100%; border-collapse: collapse; font-size: 0.95em; text-align: left;">
                        <thead>
                            <tr style="border-bottom: 1px solid var(--border-color); color: #888;">
                                <th style="padding: 10px;" data-i18n="th_torrent">Torrent (Tag)</th>
                                <th style="padding: 10px;" data-i18n="th_status">Statut</th>
                                <th style="padding: 10px;" data-i18n="th_details">Détails (Erreurs / Strikes)</th>
                                <th style="padding: 10px;" data-i18n="th_time">Temps restant</th>
                                <th style="padding: 10px;">Action</th>
                            </tr>
                        </thead>
                        <tbody id="watcher-state-body">
                            <tr><td colspan="4" style="padding: 15px; text-align: center; color: #666;" data-i18n="watcher_empty">Aucune erreur en cours ...</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>

        <div class="panel collapsed" id="panel-rules">
            <div class="collapsible-header" onclick="togglePanel('panel-rules')">
                <h2 style="display: flex; align-items: center;">
                    <span class="collapse-icon">▶</span> <span data-i18n="rules_title">📝 Éditeur de Règles</span>&nbsp;({{ rules_filename }})
                </h2>
            </div>
            <div class="collapse-content" style="margin-top: 10px;">
                <p style="font-size: 0.85em; color: #888;" data-i18n="rules_desc">Gestion visuelle structurée : Condition(s) ➔ Action ➔ Assignation/Paramètres.</p>
                <div id="visual-rules-container"></div>
                <div style="display: flex; gap: 10px; margin-top: 10px;">
                    <button onclick="addRuleRow()" data-i18n="btn_add_rule">➕ Ajouter une règle</button>
                    <button onclick="toggleRawEditor()" class="small" style="background: #555;" data-i18n="btn_raw_view">🔄 Vue Texte Brut</button>
                    <button onclick="saveRules()" style="margin-left: auto;" data-i18n="btn_save_rules">💾 Sauvegarder les règles</button>
                </div>
                <div class="raw-editor-container" id="raw-editor-container">
                    <p style="font-size: 0.85em; color: #e84d35;" data-i18n="raw_hint">Éditeur manuel (Format : condition | action | assignation)</p>
                    <textarea id="rules-editor" onchange="parseTextToVisual()"></textarea>
                </div>
            </div>
        </div>

        <div class="panel">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;">
                <h2 style="display: flex; align-items: center;">
                    <span data-i18n="logs_title">🔍 Logs</span>&nbsp;({{ logfile_filename }})
                    <div class="refresh-slot">
                        <div class="progress-container">
                            <div id="log-progress-bar" class="progress-bar"></div>
                        </div>
                        <span id="log-spinner" class="refresh-spinner" title="Mise à jour..."></span>
                    </div>
                </h2>
                <div style="display: flex; gap: 8px; align-items: center;">
                    <button onclick="clearLogs()" class="small stop" data-i18n="btn_clear">Vider</button>
                    <button onclick="archiveLogs()" class="small" style="background-color: #2e7d32;" data-i18n="btn_archive">📁 Archiver</button>
                    <button onclick="fetchLogs()" class="small" data-i18n="btn_refresh">Rafraîchir</button>
                    <label for="log-lines" style="font-size: 0.85em; color: #888;" data-i18n="lbl_lines">Lignes :</label>
                    <select id="log-lines" onchange="fetchLogs()" style="padding: 3px 6px; font-size: 0.85em;">
                        <option value="50">50</option>
                        <option value="100" selected>100</option>
                        <option value="200">200</option>
                        <option value="500">500</option>
                        <option value="1000">1000</option>
                    </select>
                </div>
            </div>
            <div id="logs"></div>
        </div>

    <datalist id="default-config-list">
        <option value="default_config.upload_rate"></option>
        <option value="default_config.download_rate"></option>
        <option value="default_config.port"></option>
        <option value="default_config.vpn_port_sync"></option>
        <option value="default_config.client_type"></option>
        <option value="default_config.client_version"></option>
        <option value="default_config.initial_uploaded"></option>
        <option value="default_config.initial_downloaded"></option>
        <option value="default_config.completion_percent"></option>
        <option value="default_config.num_want"></option>
        <option value="default_config.randomize_rates"></option>
        <option value="default_config.random_range_percent"></option>
        <option value="default_config.randomize_ratio"></option>
        <option value="default_config.random_ratio_range_percent"></option>
        <option value="default_config.stop_at_ratio"></option>
        <option value="default_config.effective_stop_at_ratio"></option>
        <option value="default_config.stop_at_uploaded"></option>
        <option value="default_config.stop_at_downloaded"></option>
        <option value="default_config.stop_at_seed_time"></option>
        <option value="default_config.idle_when_no_leechers"></option>
        <option value="default_config.idle_when_no_seeders"></option>
        <option value="default_config.scrape_interval"></option>
        <option value="default_config.progressive_rates"></option>
        <option value="default_config.target_upload_rate"></option>
        <option value="default_config.target_download_rate"></option>
        <option value="default_config.progressive_duration"></option>
        <option value="default_config.post_stop_action"></option>
    </datalist>

    <script>
        const i18n = {
            fr: {
                ctrl_title: "⚙️ Rustatio Control", daemon_check: "Vérification...", btn_daemon_start: "▶ Démarrer Daemon",
                btn_daemon_stop: "⏹ Arrêter Daemon", btn_daemon_restart: "🔄 Redémarrer Daemon", btn_script_restart: "♻️ Redémarrer Script",
                env_title: "🔧 Configuration de l'environnement (Modifications temporaires)", btn_reset: "🔄 Réinitialiser",
                btn_apply: "⚡ Appliquer à la session", env_loading: "Chargement des variables...", watcher_title: "👁️ Log Watcher",
                btn_start: "▶ Démarrer", btn_stop: "⏹ Arrêter", btn_restart: "🔄 Redémarrer", th_torrent: "Torrent (Tag)",
                th_status: "Statut", th_details: "Détails (Erreurs / Strikes)", th_time: "Temps restant", watcher_empty: "Aucune erreur en cours ...",
                rules_title: "📝 Éditeur de Règles", rules_desc: "Gestion visuelle structurée : Condition(s) ➔ Action ➔ Assignation/Paramètres.",
                btn_add_rule: "➕ Ajouter une règle", btn_raw_view: "🔄 Vue Texte Brut", btn_save_rules: "💾 Sauvegarder les règles",
                raw_hint: "Éditeur manuel (Format : condition | action | assignation)", logs_title: "🔍 Logs", btn_clear: "Vider",
                btn_archive: "📁 Archiver", btn_refresh: "Rafraîchir", lbl_lines: "Lignes :", no_params: "Aucun paramètre",
                ph_val: "Valeur ou default_config...", ph_param: "Paramètres (ex: étiquette1, étiquette2)", ph_ex: "Ex: 1.8 - 2.4",
                ph_val_only: "Valeur", rule_cond: "1. Condition(s)", btn_add_cond: "➕ Condition", btn_del: "Supprimer",
                btn_del_rule: "Supprimer la règle", rule_act: "2. Action", rule_param: "3. Paramètres / Assignation",
                daemon_running: "En cours d'exécution", daemon_stopped: "Arrêté", conn_err: "Erreur de connexion", crash: "Crashé :",
                thread_inactive: "Arrêté (Thread inactif)", expiring: "Expiration...", paused: "En pause", obs: "Observation",
                strikes: "Total Strikes :", rule_saved: "Règles sauvegardées avec succès !", confirm_clear: "Voulez-vous vraiment vider le fichier de logs ?",
                err_clear: "Erreur lors de la suppression des logs.", archive_success: "Logs archivés avec succès : ",
                archive_err: "Erreur lors de l'archivage : ", confirm_restart: "Voulez-vous vraiment redémarrer le panneau d'administration ?",
                restart_msg: "Le panneau admin redémarre...\\nLa page va se recharger dans 3 secondes.", err_restart: "Erreur lors de la demande de redémarrage.",
                read_only: "Lecture seule (redémarrage requis)", env_applied: "Configuration appliquée à la session en cours.\\nLa page va se recharger dans 3 secondes.", env_err: "Erreur",
                unauth: "Action non autorisée", confirm_reset: "Rétablir les variables de session à leurs valeurs initiales ?",
                reset_success: "Configuration réinitialisée aux valeurs de départ.\\nLa page va se recharger dans 3 secondes.", reset_err: "Erreur lors de la réinitialisation.",
                page_title: "Rustatio - Panneau de Contrôle", clearnresume: "▶ Nettoyer & Reprendre", clearnresume_msg: "Voulez-vous vraiment nettoyer les strikes et reprendre le torrent",
                clearnresume_err: "Erreur lors de la reprise du torrent.", announce: "Annonce"
            },
            en: {
                ctrl_title: "⚙️ Rustatio Control", daemon_check: "Checking...", btn_daemon_start: "▶ Start Daemon",
                btn_daemon_stop: "⏹ Stop Daemon", btn_daemon_restart: "🔄 Restart Daemon", btn_script_restart: "♻️ Restart Script",
                env_title: "🔧 Environment Configuration (Temporary overrides)", btn_reset: "🔄 Reset", btn_apply: "⚡ Apply to session",
                env_loading: "Loading variables...", watcher_title: "👁️ Log Watcher", btn_start: "▶ Start", btn_stop: "⏹ Stop",
                btn_restart: "🔄 Restart", th_torrent: "Torrent (Tag)", th_status: "Status", th_details: "Details (Errors / Strikes)",
                th_time: "Time remaining", watcher_empty: "No errors at the moment ...", rules_title: "📝 Rules Editor",
                rules_desc: "Structured visual management: Condition(s) ➔ Action ➔ Assignment/Parameters.", btn_add_rule: "➕ Add a rule",
                btn_raw_view: "🔄 Raw Text View", btn_save_rules: "💾 Save rules", raw_hint: "Manual editor (Format: condition | action | assignment)",
                logs_title: "🔍 Logs", btn_clear: "Clear", btn_archive: "📁 Archive", btn_refresh: "Refresh", lbl_lines: "Lines:",
                no_params: "No parameters", ph_val: "Value or default_config...", ph_param: "Parameters (e.g. tag1, tag2)", ph_ex: "E.g.: 1.8 - 2.4",
                ph_val_only: "Value", rule_cond: "1. Condition(s)", btn_add_cond: "➕ Condition", btn_del: "Delete",
                btn_del_rule: "Delete rule", rule_act: "2. Action", rule_param: "3. Parameters / Assignment",
                daemon_running: "Running", daemon_stopped: "Stopped", conn_err: "Connection error", crash: "Crashed:",
                thread_inactive: "Stopped (Inactive thread)", expiring: "Expiring...", paused: "Paused", obs: "Observing",
                strikes: "Total Strikes :", rule_saved: "Rules successfully saved!", confirm_clear: "Are you sure you want to clear the logs file?",
                err_clear: "Error while clearing logs.", archive_success: "Logs successfully archived: ",
                archive_err: "Error archiving logs: ", confirm_restart: "Are you sure you want to restart the administration panel?",
                restart_msg: "Admin panel is restarting...\\nThe page will reload in 3 seconds.", err_restart: "Error while requesting restart.",
                read_only: "Read-only (requires restart)", env_applied: "Configuration applied to current session.\\nThe page will reload in 3 seconds.", env_err: "Error",
                unauth: "Unauthorized action", confirm_reset: "Reset session variables to their initial values?",
                reset_success: "Configuration reset to default values.\\nThe page will reload in 3 seconds.", reset_err: "Error while resetting configuration.",
                page_title: "Rustatio - Control Panel", clearnresume: "▶ Clear & Resume", clearnresume_msg: "Would you like to clear strikes and resume torrent",
                clearnresume_err: "Failed to resume torrent.", announce: "Announce"
            }
        };

        let currentLang = localStorage.getItem('rustatio_lang') || 'fr';

        function t(key) {
            return (i18n[currentLang] && i18n[currentLang][key]) ? i18n[currentLang][key] : key;
        }

        function setLang(lang) {
            currentLang = lang;
            localStorage.setItem('rustatio_lang', lang);
            
            document.querySelectorAll('[data-i18n]').forEach(el => {
                const key = el.getAttribute('data-i18n');
                if (el.tagName === 'INPUT' && el.type === 'button') {
                    el.value = t(key);
                } else {
                    el.textContent = t(key);
                }
            });
            
            document.title = t('page_title');
            
            const selectElem = document.getElementById('lang-select');
            if (selectElem && selectElem.value !== lang) {
                selectElem.value = lang;
            }

            if(typeof fetchStatus === 'function') fetchStatus();
            if(typeof fetchWatcherStatus === 'function') fetchWatcherStatus();
            if(typeof fetchWatcherState === 'function') fetchWatcherState();
            if(typeof fetchEnvConfig === 'function') fetchEnvConfig();
            
            const raw = document.getElementById('raw-editor-container');
            if (raw && raw.style.display !== 'block' && typeof parseTextToVisual === 'function') {
                parseTextToVisual();
            }
        }

        const FIELDS = [
            "id", "torrent.info_hash", "torrent.announce", "torrent.name", "torrent.total_size",
            "torrent.piece_length", "torrent.num_pieces", "torrent.comment", "torrent.created_by",
            "torrent.is_single_file", "torrent.file_count", 
            "config.upload_rate", "config.download_rate", "config.port", "config.vpn_port_sync",
            "config.client_type", "config.client_version", "config.initial_uploaded", "config.initial_downloaded",
            "config.completion_percent", "config.num_want", "config.randomize_rates", "config.random_range_percent",
            "config.randomize_ratio", "config.random_ratio_range_percent", "config.stop_at_ratio",
            "config.effective_stop_at_ratio", "config.stop_at_uploaded", "config.stop_at_downloaded",
            "config.stop_at_seed_time", "config.idle_when_no_leechers", "config.idle_when_no_seeders",
            "config.scrape_interval", "config.progressive_rates", "config.target_upload_rate",
            "config.target_download_rate", "config.progressive_duration", "config.post_stop_action",
            "stats.uploaded", "stats.downloaded", "stats.ratio", "stats.left", "stats.torrent_completion",
            "stats.seeders", "stats.leechers", "stats.state", "stats.is_idling", "stats.idling_reason",
            "stats.session_uploaded", "stats.session_downloaded", "stats.session_ratio",
            "stats.current_upload_rate", "stats.current_download_rate", "stats.average_upload_rate",
            "stats.average_download_rate", "stats.upload_progress", "stats.download_progress",
            "stats.ratio_progress", "stats.seed_time_progress", "stats.effective_stop_at_ratio",
            "stats.eta_ratio", "stats.eta_uploaded", "stats.eta_download_completion",
            "stats.stop_condition_met", "stats.post_stop_action", "stats.stats.announce_count",
            "tags", "default_config.upload_rate", "default_config.download_rate"
        ];
        
        const CONFIG_TYPES = {
            "config.upload_rate": "number", "config.download_rate": "number", "config.port": "number",
            "config.vpn_port_sync": "boolean", "config.client_type": "string", "config.client_version": "string",
            "config.initial_uploaded": "number", "config.initial_downloaded": "number",
            "config.completion_percent": "number", "config.num_want": "number", "config.randomize_rates": "boolean",
            "config.random_range_percent": "number", "config.randomize_ratio": "boolean",
            "config.random_ratio_range_percent": "number", "config.stop_at_ratio": "number",
            "config.effective_stop_at_ratio": "number", "config.stop_at_uploaded": "number",
            "config.stop_at_downloaded": "number", "config.stop_at_seed_time": "number",
            "config.idle_when_no_leechers": "boolean", "config.idle_when_no_seeders": "boolean",
            "config.scrape_interval": "number", "config.progressive_rates": "boolean",
            "config.target_upload_rate": "number", "config.target_download_rate": "number",
            "config.progressive_duration": "number", "config.post_stop_action": "string"
        };
        
        const OPERATORS = [":", "=", "<", ">", "!=", "<=", ">=", "~"];
        const ACTIONS = ["start", "stop", "pause", "resume", "delete", "update", "addtags", "removetags"];
        const LOGICAL_OPS = ["AND", "OR"];

        let currentReadonlyKeys = [];

        function createOptions(arr, selected = '') {
            return arr.map(item => `<option value="${item}" ${item === selected ? 'selected' : ''}>${item}</option>`).join('');
        }
        
        function createConfigOptions(selected = '') {
            return Object.keys(CONFIG_TYPES).map(k => `<option value="${k}" ${k === selected ? 'selected' : ''}>${k}</option>`).join('');
        }

        function getActionValueInput(configKey, val) {
            return `<input type="text" class="action-assign-val" list="default-config-list" placeholder="${t('ph_val')}" value="${val.replace(/"/g, '&quot;')}">`;
        }

        function applyActionStyle(selectElem) {
            const action = selectElem.value;
            selectElem.setAttribute('data-action', action);
        }

        function updateActionUI(selectElem) {
            applyActionStyle(selectElem);
            const action = selectElem.value;
            const row = selectElem.closest('.rule-row');
            const container = row.querySelector('.action-assign-container');
            const assignBlock = row.querySelector('.block-assign');
            const assignArrow = row.querySelector('.arrow-assign');

            if (['start', 'stop', 'pause', 'resume'].includes(action)) {
                container.innerHTML = `<span style="color: #666; font-size: 0.85em; font-style: italic;">${t('no_params')}</span>`;
                assignBlock.style.opacity = '0.5';
                if (assignArrow) assignArrow.style.opacity = '0.3';
            } else {
                assignBlock.style.opacity = '1';
                if (assignArrow) assignArrow.style.opacity = '1';

                if (action === 'update') {
                    const confKey = 'config.upload_rate';
                    container.innerHTML = `
                        <select class="action-assign-conf" style="flex-grow:1;">${createConfigOptions(confKey)}</select>
                        <span style="color:#888; font-weight:bold; align-self:center;">=</span>
                        ${getActionValueInput(confKey, '')}
                    `;
                } else {
                    container.innerHTML = `<input type="text" class="action-assign-text" placeholder="${t('ph_param')}" value="">`;
                }
            }
        }

        function renderActionAssignContainer(action, assignVal) {
            if (['start', 'stop', 'pause', 'resume'].includes(action)) {
                return `<span style="color: #666; font-size: 0.85em; font-style: italic;">${t('no_params')}</span>`;
            } else if (action === 'update') {
                let confKey = 'config.upload_rate';
                let val = '';
                if (assignVal) {
                    const parts = assignVal.split('=');
                    if (parts.length >= 2) {
                        confKey = parts[0].trim();
                        val = parts.slice(1).join('=').trim();
                    }
                }
                if (!CONFIG_TYPES[confKey]) confKey = 'config.upload_rate';
                
                return `
                    <select class="action-assign-conf" style="flex-grow:1;">${createConfigOptions(confKey)}</select>
                    <span style="color:#888; font-weight:bold; align-self:center;">=</span>
                    ${getActionValueInput(confKey, val)}
                `;
            } else {
                return `<input type="text" class="action-assign-text" placeholder="${t('ph_param')}" value="${(assignVal||'').replace(/"/g, '&quot;')}">`;
            }
        }

        function addCondition(container, logOp='', fieldVal='', opVal=':', condVal='') {
            const block = document.createElement('div');
            block.className = 'condition-block';
            let logOpHtml = '';
            if (container.children.length > 0) {
                logOpHtml = `<select class="cond-logop">${createOptions(LOGICAL_OPS, logOp || 'AND')}</select>`;
            }
            const placeholder = opVal === ':' ? t('ph_ex') : t('ph_val_only');

            block.innerHTML = `
                ${logOpHtml}
                <select class="cond-field">${createOptions(FIELDS, fieldVal)}</select>
                <select class="cond-op">${createOptions(OPERATORS, opVal)}</select>
                <input type="text" class="cond-val" placeholder="${placeholder}" value="${condVal.replace(/"/g, '&quot;')}">
                ${container.children.length > 0 ? `<button class="stop small" onclick="this.parentElement.remove()" title="${t('btn_del')}">✕</button>` : ''}
            `;
            container.appendChild(block);
        }

        function addRuleRow(conditions = [], actionVal='start', assignVal='') {
            const container = document.getElementById('visual-rules-container');
            const row = document.createElement('div');
            row.className = 'rule-row';
            const isSimpleAction = ['start', 'stop', 'pause', 'resume'].includes(actionVal);

            row.innerHTML = `
                <div class="rule-block block-conditions">
                    <div class="rule-block-title">${t('rule_cond')}</div>
                    <div class="conditions-wrapper"></div>
                    <button class="small" onclick="addCondition(this.previousElementSibling)" style="align-self: flex-start; margin-top: 4px;">➕ ${t('btn_add_cond')}</button>
                </div>
                <div class="rule-arrow">➔</div>
                <div class="rule-block block-action">
                    <div class="rule-block-title">${t('rule_act')}</div>
                    <select class="action-type" data-action="${actionVal}" onchange="updateActionUI(this)">${createOptions(ACTIONS, actionVal)}</select>
                </div>
                <div class="rule-arrow arrow-assign" style="${isSimpleAction ? 'opacity: 0.3;' : ''}">➔</div>
                <div class="rule-block block-assign" style="${isSimpleAction ? 'opacity: 0.5;' : ''}">
                    <div class="rule-block-title">${t('rule_param')}</div>
                    <div class="action-assign-container" style="display: flex; gap: 6px; align-items: center; flex-grow: 1;">
                        ${renderActionAssignContainer(actionVal, assignVal)}
                    </div>
                </div>
                <button class="btn-delete-rule" onclick="this.closest('.rule-row').remove()" title="${t('btn_del_rule')}">✕</button>
            `;
            
            const condWrapper = row.querySelector('.conditions-wrapper');
            if (conditions.length === 0) {
                addCondition(condWrapper);
            } else {
                conditions.forEach(c => addCondition(condWrapper, c.logOp, c.field, c.op, c.val));
            }
            container.appendChild(row);
        }

        function parseVisualToText() {
            const rows = document.querySelectorAll('.rule-row');
            let text = [];
            rows.forEach(row => {
                const condBlocks = row.querySelectorAll('.condition-block');
                let condString = '';
                condBlocks.forEach((block, index) => {
                    const field = block.querySelector('.cond-field').value;
                    const op = block.querySelector('.cond-op').value;
                    const val = block.querySelector('.cond-val').value;
                    if (index > 0) condString += ` ${block.querySelector('.cond-logop').value} `;
                    condString += op === ':' ? `${field}: ${val}` : `${field} ${op} ${val}`;
                });
                
                const action = row.querySelector('.action-type').value;
                let assign = '';
                const assignContainer = row.querySelector('.action-assign-container');
                if (action === 'update') {
                    const conf = assignContainer.querySelector('.action-assign-conf')?.value;
                    const val = assignContainer.querySelector('.action-assign-val')?.value;
                    if (conf && val !== undefined) assign = `${conf} = ${val}`;
                } else if (!['start', 'stop', 'pause', 'resume'].includes(action)) {
                    const txt = assignContainer.querySelector('.action-assign-text');
                    if (txt) assign = txt.value;
                }
                
                if (condString && action) {
                    let line = `${condString} | ${action} |`;
                    if (assign) line += ` ${assign}`;
                    text.push(line);
                }
            });
            document.getElementById('rules-editor').value = text.join('\\n');
        }

        function parseTextToVisual() {
            const container = document.getElementById('visual-rules-container');
            if (!container) return;
            container.innerHTML = '';
            const text = document.getElementById('rules-editor').value;
            const lines = text.split('\\n');
            const condRegex = /^([a-zA-Z0-9_.]+)\\s*(<=|>=|!=|:|=|<|>|~)\\s*(.*)$/;
            
            lines.forEach(line => {
                const trimmed = line.trim();
                if (!trimmed || trimmed.startsWith('#')) return; 
                
                const parts = trimmed.split('|').map(p => p.trim());
                if (parts.length >= 2) {
                    const condString = parts[0];
                    const action = parts[1] || 'start';
                    const assign = parts.length >= 3 ? parts[2] : '';
                    const tokens = condString.split(/\\s+(AND|OR|and|or)\\s+/);
                    const conditions = [];
                    let currentLogOp = '';
                    
                    tokens.forEach(token => {
                        const t = token.trim();
                        if (/^(AND|OR)$/i.test(t)) {
                            currentLogOp = t.toUpperCase();
                        } else if (t) {
                            const match = t.match(condRegex);
                            if (match) {
                                conditions.push({ logOp: currentLogOp, field: match[1], op: match[2], val: match[3] });
                            } else {
                                conditions.push({ logOp: currentLogOp, field: t, op: ':', val: '' });
                            }
                            currentLogOp = '';
                        }
                    });
                    addRuleRow(conditions, action, assign);
                }
            });
            if (container.children.length === 0) addRuleRow();
        }

        function resetAndStartProgress(barId, durationMs) {
            const bar = document.getElementById(barId);
            if (!bar) return;
            bar.style.transition = 'none';
            bar.style.width = '0%';
            void bar.offsetWidth;
            bar.style.transition = `width ${durationMs}ms linear`;
            bar.style.width = '100%';
        }

        function toggleRawEditor() {
            const raw = document.getElementById('raw-editor-container');
            if (raw.style.display === 'block') {
                raw.style.display = 'none';
                parseTextToVisual();
            } else {
                parseVisualToText();
                raw.style.display = 'block';
            }
        }
        
        function togglePanel(panelId) {
            const panel = document.getElementById(panelId);
            if (!panel) return;
            const icon = panel.querySelector('.collapse-icon');
            panel.classList.toggle('collapsed');
            if (icon) {
                icon.textContent = panel.classList.contains('collapsed') ? '▶' : '▼';
            }
        }

        async function fetchStatus() {
            const badge = document.getElementById('daemon-status');
            const btnStart = document.getElementById('btn-daemon-start');
            const btnStop = document.getElementById('btn-daemon-stop');
            const btnRestart = document.getElementById('btn-daemon-restart');
            try {
                const res = await fetch('/api/status');
                if (!res.ok) throw new Error('HTTP ' + res.status);
                const data = await res.json();
                const isRunning = !!data.running;
                if (isRunning) {
                    badge.className = 'status-badge status-running';
                    badge.innerText = `${t('daemon_running')} (PID: ${data.pid})`;
                } else {
                    badge.className = 'status-badge status-stopped';
                    badge.innerText = t('daemon_stopped');
                }
                if (btnStart) btnStart.disabled = isRunning;
                if (btnStop) btnStop.disabled = !isRunning;
                if (btnRestart) btnRestart.disabled = !isRunning;
            } catch (err) {
                badge.className = 'status-badge status-stopped';
                badge.innerText = t('conn_err');
                if (btnStart) btnStart.disabled = false;
                if (btnStop) btnStop.disabled = true;
                if (btnRestart) btnRestart.disabled = true;
            }
        }

        async function daemonAction(action) {
            try { await fetch(`/api/daemon/${action}`, { method: 'POST' }); } 
            catch (e) { console.error(e); }
            setTimeout(fetchStatus, 1000);
            setTimeout(fetchLogs, 1000);
        }

        async function fetchWatcherStatus() {
            const badge = document.getElementById('watcher-status');
            const spinner = document.getElementById('watcher-spinner');
            const btnStart = document.getElementById('btn-watcher-start');
            const btnStop = document.getElementById('btn-watcher-stop');
            const btnRestart = document.getElementById('btn-watcher-restart');
            if (spinner) spinner.classList.add('active');
            try {
                const res = await fetch('/api/watcher/status');
                if (!res.ok) throw new Error('HTTP ' + res.status);
                const data = await res.json();
                const isRunning = !!data.running;
                if (isRunning) {
                    badge.className = 'status-badge status-running';
                    badge.innerText = t('daemon_running');
                } else if (data.status === 'crashed') {
                    badge.className = 'status-badge status-stopped';
                    badge.innerText = `${t('crash')} ${data.error}`;
                } else {
                    badge.className = 'status-badge status-stopped';
                    badge.innerText = t('thread_inactive');
                }
                if (btnStart) btnStart.disabled = isRunning;
                if (btnStop) btnStop.disabled = !isRunning;
                if (btnRestart) btnRestart.disabled = !isRunning;
            } catch (err) {
                badge.className = 'status-badge status-stopped';
                badge.innerText = t('conn_err');
                if (btnStart) btnStart.disabled = false;
                if (btnStop) btnStop.disabled = true;
                if (btnRestart) btnRestart.disabled = true;
            } finally {
                setTimeout(() => { if (spinner) spinner.classList.remove('active'); }, 500);
            }
        }

        async function watcherAction(action) {
            try { await fetch(`/api/watcher/${action}`, { method: 'POST' }); } 
            catch (e) { console.error(e); }
            setTimeout(fetchWatcherStatus, 1000);
            setTimeout(fetchLogs, 1000);
        }

        function formatTimeLeft(seconds) {
            if (seconds <= 0) return t('expiring');
            const h = Math.floor(seconds / 3600);
            const m = Math.floor((seconds % 3600) / 60);
            const s = seconds % 60;
            if (h > 0) return `${h}h ${m}m ${s}s`;
            return `${m}m ${s}s`;
        }

        function toggleTokenVisibility() {
            const input = document.getElementById('env-input-AUTH_TOKEN');
            const btn = document.getElementById('auth-token-btn');
            if (input) {
                if (input.type === 'password') {
                    input.type = 'text';
                    if (btn) btn.textContent = '🙈';
                } else {
                    input.type = 'password';
                    if (btn) btn.textContent = '👁️';
                }
            }
        }

        async function fetchWatcherState() {
            const spinner = document.getElementById('watcher-spinner');
            if (spinner) spinner.classList.add('active');
            try {
                const res = await fetch('/api/watcher/state');
                if (!res.ok) return;
                const data = await res.json();
                const tbody = document.getElementById('watcher-state-body');
                
                if (Object.keys(data.state).length === 0) {
                    tbody.innerHTML = `<tr><td colspan="4" style="padding: 15px; text-align: center; color: #666;">${t('watcher_empty')}</td></tr>`;
                    return;
                }
                
                let html = '';
                for (const [tag, info] of Object.entries(data.state)) {
                    let errDetails = '';
                    for (const [err, count] of Object.entries(info.errors)) {
                        errDetails += `<div style="font-size: 0.85em; color: #aaa; margin-top: 4px;">- ${err}: <strong style="color:#fff;">${count}</strong></div>`;
                    }
                    
                    let statusBadge = info.status === "paused"
                        ? `<span class="status-badge status-stopped" style="background: rgba(230, 81, 0, 0.2); color: #ffb74d;">${t('paused')}</span>`
                        : info.status === "announce"
                        ? `<span class="status-badge status-announce" style="background: rgba(33, 150, 243, 0.2); color: #64b5f6;">${t('announce')}</span>`
                        : `<span class="status-badge status-running" style="color: #a5d6a7;">${t('obs')}</span>`;
                        
                    html += `<tr style="border-bottom: 1px solid #333;">
                        <td style="padding: 10px; word-break: break-all; max-width: 250px;"><strong>${tag}</strong></td>
                        <td style="padding: 10px;">${statusBadge}</td>
                        <td style="padding: 10px;">
                            <span style="color: var(--rust-orange); font-weight: bold;">${t('strikes')} ${info.total_strikes}</span>
                            ${errDetails}
                        </td>
                        <td style="padding: 10px; color: #64b5f6; font-weight: bold;">${formatTimeLeft(info.time_left)}</td>
                        <td style="padding: 10px;">
                            <button class="small start" style="background-color: #2e7d32;" onclick="clearAndResume(this.dataset.tag)" data-tag="${tag.replace(/"/g, '&quot;')}">${t('clearnresume')}</button>
                        </td>
                    </tr>`;
                }
                tbody.innerHTML = html;
            } catch (err) {
                console.error(err);
            } finally {
                setTimeout(() => { if (spinner) spinner.classList.remove('active'); }, 500);
            }
        }

        async function clearAndResume(tag) {
            if (!confirm(`${t('clearnresume_msg')}:\\n${tag} ?`)) return;
            try {
                const res = await fetch('/api/watcher/clear_resume', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ tag })
                });
                if (res.ok) {
                    setTimeout(refreshWatcher, 1000);
                } else {
                t('clearnresume_err')
                    alert(t('clearnresume_err'));
                }
            } catch (e) { 
                console.error(e); 
            }
        }

        async function loadRules() {
            try {
                const res = await fetch('/api/rules');
                const data = await res.json();
                document.getElementById('rules-editor').value = data.content || '';
                parseTextToVisual();
            } catch (e) { console.error(e); }
        }

        async function saveRules() {
            if (document.getElementById('raw-editor-container').style.display !== 'block') {
                parseVisualToText();
            }
            const content = document.getElementById('rules-editor').value;
            const res = await fetch('/api/rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content })
            });
            if (res.ok) alert(t('rule_saved'));
        }

        async function refreshWatcher() {
            await fetchWatcherStatus();
            await fetchWatcherState();
            resetAndStartProgress('watcher-progress-bar', 3000);
        }

        async function fetchLogs() {
            const spinner = document.getElementById('log-spinner');
            const linesCount = document.getElementById('log-lines')?.value || 100;
            if (spinner) spinner.classList.add('active');
            try {
                const res = await fetch(`/api/logs?lines=${linesCount}`);
                const data = await res.json();
                const logDiv = document.getElementById('logs');
                const isAtBottom = logDiv.scrollHeight - logDiv.clientHeight - logDiv.scrollTop <= 30;
                const scrollOffsetFromBottom = logDiv.scrollHeight - logDiv.scrollTop;
                logDiv.textContent = data.content;
                if (isAtBottom) {
                    logDiv.scrollTop = logDiv.scrollHeight;
                } else {
                    logDiv.scrollTop = logDiv.scrollHeight - scrollOffsetFromBottom;
                }
            } catch (e) { 
                console.error(e); 
            } finally {
                setTimeout(() => {
                    if (spinner) spinner.classList.remove('active');
                    resetAndStartProgress('log-progress-bar', 3000);
                }, 500);
            }
        }

        async function clearLogs() {
            if (!confirm(t('confirm_clear'))) return;
            try {
                const res = await fetch('/api/logs/clear', { method: 'POST' });
                if (res.ok) fetchLogs();
                else alert(t('err_clear'));
            } catch (e) { console.error(e); }
        }

        async function archiveLogs() {
            try {
                const res = await fetch('/api/logs/archive', { method: 'POST' });
                const data = await res.json();
                if (res.ok && data.success) {
                    alert(t('archive_success') + data.filename);
                    fetchLogs();
                } else {
                    alert(t('archive_err') + (data.error || ""));
                }
            } catch (e) { console.error(e); }
        }

        async function restartAdmin() {
            if (!confirm(t('confirm_restart'))) return;
            try {
                await fetch('/api/admin/restart', { method: 'POST' });
                alert(t('restart_msg'));
                setTimeout(() => window.location.reload(), 3000);
            } catch (e) {
                console.error(e);
                alert(t('err_restart'));
            }
        }

        async function fetchEnvConfig() {
            try {
                const res = await fetch('/api/env');
                if (!res.ok) return;
                const data = await res.json();
                const container = document.getElementById('env-config-grid');
                if (!container) return;

                const config = data.config || {};
                currentReadonlyKeys = data.readonly || [];

                let html = '';
                for (const [key, val] of Object.entries(config)) {
                    let inputHtml = '';
                    const isReadOnly = currentReadonlyKeys.includes(key);

                    const cardStyle = isReadOnly 
                        ? 'background: #121212; border: 1px dashed #333; opacity: 0.75;' 
                        : 'background: #181818; border: 1px solid var(--border-color);';
                    
                    const labelHtml = isReadOnly 
                        ? `<span style="color: #777; font-family: monospace;" title="${t('read_only')}">🔒 ${key}</span>` 
                        : `<span style="color: #aaa; font-family: monospace;">${key}</span>`;

                    if (key === 'AUTH_TOKEN') {
                        inputHtml = `
                            <div style="display: inline-flex; align-items: center; gap: 4px;">
                                <input id="env-input-${key}" type="password" value="${val}" ${isReadOnly ? 'readonly' : ''} 
                                       style="background: #181818; border: 1px solid #333; border-radius: 4px; color: #888; font-family: monospace; width: 100px; padding: 2px 5px; text-align: right; ${isReadOnly ? 'cursor: not-allowed;' : ''}">
                                <button id="auth-token-btn" onclick="toggleTokenVisibility()" type="button"
                                        style="background: none; border: none; cursor: pointer; padding: 0; font-size: 1em;">👁️</button>
                            </div>`;
                    } else if (key === 'RUST_DAEMON_LOG') {
                        const currentVal = (val || '').toString().toUpperCase();
                        const levels = ['', 'TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR'];
                        const options = levels.map(l => `<option value="${l}" ${currentVal === l ? 'selected' : ''}>${l || '(Default)'}</option>`).join('');
                        inputHtml = `
                            <select id="env-input-${key}" style="background: #222; border: 1px solid #444; border-radius: 4px; color: #fff; font-family: monospace; padding: 2px 5px;">
                                ${options}
                            </select>`;
                    } else if (key === 'LOGS_WATCHER' || key === 'TOR_KEEP_LAST') {
                        const intVal = parseInt(val, 10) || 0;
                        inputHtml = `
                            <select id="env-input-${key}" style="background: #222; border: 1px solid #444; border-radius: 4px; color: #fff; font-family: monospace; padding: 2px 5px;">
                                <option value="0" ${intVal === 0 ? 'selected' : ''}>0</option>
                                <option value="1" ${intVal === 1 ? 'selected' : ''}>1</option>
                            </select>`;
                    } else if (key === 'DRY_RUN' || typeof val === 'boolean') {
                        const boolVal = (val === true || val === 'true');
                        inputHtml = `
                            <select id="env-input-${key}" style="background: #222; border: 1px solid #444; border-radius: 4px; color: #fff; font-family: monospace; padding: 2px 5px;">
                                <option value="true" ${boolVal ? 'selected' : ''}>true</option>
                                <option value="false" ${!boolVal ? 'selected' : ''}>false</option>
                            </select>`;
                    } else {
                        const inputBg = isReadOnly ? '#181818' : '#222';
                        const inputBorder = isReadOnly ? '1px solid #333' : '1px solid #444';
                        const inputColor = isReadOnly ? '#888' : '#fff';
                        const inputCursor = isReadOnly ? 'text' : 'text';
                        const isNumber = typeof val === 'number';
                        const numberAttrs = isNumber ? 'min="0" step="1"' : '';

                        inputHtml = `
                            <input id="env-input-${key}" type="${isNumber ? 'number' : 'text'}" value="${val}" ${numberAttrs} ${isReadOnly ? 'readonly' : ''} 
                                   style="background: ${inputBg}; border: ${inputBorder}; border-radius: 4px; color: ${inputColor}; font-family: monospace; width: 110px; padding: 2px 5px; text-align: right; cursor: ${inputCursor};">`;
                    }

                    html += `
                        <div style="${cardStyle} padding: 8px 12px; border-radius: 6px; display: flex; justify-content: space-between; align-items: center; font-size: 0.85em;">
                            ${labelHtml}
                            <div style="margin-left: 10px;">${inputHtml}</div>
                        </div>`;
                }
                container.innerHTML = html;
            } catch (e) {
                console.error(e);
            }
        }

        async function saveEnvConfig() {
            const payload = {};
            let invalidKey = null;

            document.querySelectorAll('[id^="env-input-"]').forEach(el => {
                if (invalidKey) return;
                const key = el.id.replace('env-input-', '');
                if (!currentReadonlyKeys.includes(key)) {
                    if (key === 'LOGS_WATCHER' || key === 'TOR_KEEP_LAST') {
                        payload[key] = parseInt(el.value, 10);
                    } else if (el.value === 'true' || el.value === 'false') {
                        payload[key] = el.value === 'true';
                    } else if (el.type === 'number') {
                        const parsed = parseInt(el.value, 10);
                        if (isNaN(parsed) || parsed < 0) {
                            invalidKey = key;
                            return;
                        }
                        payload[key] = parsed;
                    } else {
                        payload[key] = el.value;
                    }
                }
            });

            if (invalidKey) {
                alert(`Valeur invalide pour ${invalidKey}. Veuillez saisir un nombre entier supérieur ou égal à 0.`);
                return;
            }

            try {
                const res = await fetch('/api/env', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
                
                const data = await res.json();

                if (res.ok) {
                    alert(t('env_applied'));
                    fetchEnvConfig();
                    setTimeout(() => window.location.reload(), 3000);
                } else {
                    alert(`${t('env_err')} (${res.status}) : ${data.message || t('unauth')}`);
                }
            } catch (e) {
                console.error(e);
            }
        }

        async function resetEnvConfig() {
            if (!confirm(t('confirm_reset'))) return;

            try {
                const res = await fetch('/api/env/reset', { method: 'POST' });
                if (res.ok) {
                    await fetchEnvConfig();
                    alert(t('reset_success'));
                    setTimeout(() => window.location.reload(), 3000);
                } else {
                    alert(t('reset_err'));
                }
            } catch (e) {
                console.error(e);
            }
        }

        setLang(currentLang);
        loadRules();
        fetchStatus();
        refreshWatcher();
        fetchLogs();
        fetchEnvConfig();
        setInterval(fetchStatus, 3000);
        setInterval(refreshWatcher, 3000);
        setInterval(fetchLogs, 3000);
    </script>
</body>
</html>
"""

# ==========================================
# LOGGING SETUP
# ==========================================
RUST_LOG_LEVELS = {
    "TRACE": logging.DEBUG,
    "DEBUG": logging.DEBUG,
    "INFO": logging.INFO,
    "WARN": logging.WARNING,
    "ERROR": logging.ERROR,
}

def get_global_rust_log_level(env_val):
    if not env_val:
        return logging.INFO
        
    directives = [d.strip() for d in env_val.split(",") if d.strip()]
    for directive in directives:
        if "=" not in directive:
            level_str = directive.upper()
            if level_str in RUST_LOG_LEVELS:
                return RUST_LOG_LEVELS[level_str]
    
    return logging.INFO

daemon_log_env = os.getenv("RUST_DAEMON_LOG")

if daemon_log_env:
    log_level = RUST_LOG_LEVELS.get(daemon_log_env.strip().upper(), logging.INFO)
else:
    rust_log_env = os.getenv("RUST_LOG", "info")
    log_level = get_global_rust_log_level(rust_log_env)

logger = logging.getLogger("Rustatio")
logger.setLevel(log_level)

formatter = logging.Formatter('%(asctime)s :: %(message)s', "%d-%m-%Y %H:%M:%S")
console_handler = logging.StreamHandler()
console_handler.setFormatter(formatter)
logger.addHandler(console_handler)

class FlushingRotatingFileHandler(RotatingFileHandler):
    def emit(self, record):
        super().emit(record)
        self.flush()

def setup_file_handler():
    if LOGFILE and LOGFILE != "/dev/null":
        for h in logger.handlers[:]:
            if isinstance(h, RotatingFileHandler):
                h.close()
                logger.removeHandler(h)
        log_dir = os.path.dirname(LOGFILE)
        if log_dir:
            os.makedirs(log_dir, exist_ok=True)
        if not os.path.exists(LOGFILE):
            open(LOGFILE, 'a').close()
        
        file_handler = FlushingRotatingFileHandler(LOGFILE, maxBytes=1024*1024, backupCount=10)
        file_handler.setFormatter(formatter)
        logger.addHandler(file_handler)

setup_file_handler()

STYLE_LEVELS = {
    "trace": logging.DEBUG, "finish": logging.INFO, "task": logging.INFO,
    "recycle": logging.INFO, "lock": logging.INFO, "data": logging.INFO,
    "saving": logging.INFO, "succes": logging.INFO, "start": logging.INFO,
    "pinned": logging.INFO, "warning": logging.WARNING, "watcher": logging.WARNING,
    "marked": logging.WARNING, "error": logging.ERROR, "denied": logging.ERROR
}

def log(msg, style="default"):
    prefix_spaces = ""
    base_style = style

    if style.startswith("ff_"):
        prefix_spaces = "          └─ "
        base_style = style[3:]
    elif style.startswith("f_"):
        prefix_spaces = "   └─ "
        base_style = style[2:]

    prefixes = {
        "start": "🚀 ", "error": "❌ ", "succes": "✅️ ", "warning": "⚠️ ",
        "denied": "🚫 ", "saving": "💾 ", "data": "🧪 ", "lock": "🔒 ",
        "recycle": "♻️ ", "task": "⚡ ", "finish": "🏁 ", "trace": "🔍 ",
        "watcher": "👁 ", "pinned": "📍 ", "marked": "🔖 "
    }
    prefix = prefixes.get(base_style, "")
    level = STYLE_LEVELS.get(base_style, logging.INFO)
    logger.log(level, f"{prefix_spaces}{prefix}{msg}")


# ==========================================
# API CLIENT
# ==========================================
class APIClient:
    def __init__(self, base_url):
        self.base_url = base_url.rstrip("/")
        self.session = requests.Session()
        self.session.headers.update({"Accept": "application/json"})

        if AUTH_TOKEN:
            self.session.headers.update({"Authorization": f"Bearer {AUTH_TOKEN}"})

        log(f"APIClient initialized with base_url: {self.base_url}", "trace")

    def request(self, method, endpoint, payload=None, timeout=5):
        url = f"{self.base_url}/api/{endpoint}"
        max_retries = 3
        log(f"API Request -> {method} {url} with payload: {payload}", "trace")

        for attempt in range(1, max_retries + 1):
            try:
                if payload and method in ["POST", "PATCH"]:
                    resp = self.session.request(method, url, json=payload, timeout=timeout)
                else:
                    resp = self.session.request(method, url, timeout=timeout)
                resp.raise_for_status()
                data = resp.json()
                log(f"API Response <- {method} {endpoint} Status: {resp.status_code}", "trace")

                if data.get("success") is True:
                    if "data" in data or "stats" in data or "config" in data:
                        return data
                    return None
                elif "data" in data or isinstance(data, list):
                    return data
            except Exception as e:
                if attempt == max_retries:
                    log(f"Failed to {method} {endpoint} after {max_retries} attempts. Error: {str(e)}", "f_error")
                else:
                    log(f"Attempt {attempt} failed for {method} {endpoint}. Retrying in 2 seconds...", "f_error")
                    time.sleep(2)
        return None


# ==========================================
# AST EVALUATOR (REPLACES EVAL SAFELY)
# ==========================================
class ASTEvaluator:
    BINARY_OPS = {
        ast.Add: operator.add,
        ast.Sub: operator.sub,
        ast.Mult: operator.mul,
        ast.Div: operator.truediv,
        ast.FloorDiv: operator.floordiv,
        ast.Mod: operator.mod,
        ast.Pow: operator.pow,
    }

    UNARY_OPS = {
        ast.UAdd: operator.pos,
        ast.USub: operator.neg,
        ast.Not: operator.not_,
    }

    COMP_OPS = {
        ast.Eq: operator.eq,
        ast.NotEq: operator.ne,
        ast.Lt: operator.lt,
        ast.LtE: operator.le,
        ast.Gt: operator.gt,
        ast.GtE: operator.ge,
        ast.In: lambda a, b: a in b,
        ast.NotIn: lambda a, b: a not in b,
    }

    ALLOWED_FUNCS = {
        "str": str,
        "float": float,
        "int": int,
        "bool": bool,
    }

    ALLOWED_METHODS = {"lower", "upper", "strip", "hex", "startswith", "endswith"}

    def __init__(self, context):
        self.context = context

    def eval(self, node):
        if isinstance(node, ast.Expression):
            return self.eval(node.body)

        elif isinstance(node, ast.Constant):
            return node.value

        elif hasattr(ast, 'Num') and isinstance(node, ast.Num):
            return node.n
        elif hasattr(ast, 'Str') and isinstance(node, ast.Str):
            return node.s
        elif hasattr(ast, 'NameConstant') and isinstance(node, ast.NameConstant):
            return node.value
        elif hasattr(ast, 'Bytes') and isinstance(node, ast.Bytes):
            return node.s

        elif isinstance(node, ast.Name):
            if node.id in ("True", "true"):
                return True
            if node.id in ("False", "false"):
                return False
            if node.id in ("None", "null"):
                return None
            if node.id in self.context:
                return self.context[node.id]
            if node.id in self.ALLOWED_FUNCS:
                return self.ALLOWED_FUNCS[node.id]
            raise NameError(f"Name '{node.id}' is not defined in evaluation context")

        elif isinstance(node, ast.BoolOp):
            if isinstance(node.op, ast.And):
                for val in node.values:
                    res = self.eval(val)
                    if not res:
                        return res
                return res
            elif isinstance(node.op, ast.Or):
                for val in node.values:
                    res = self.eval(val)
                    if res:
                        return res
                return res

        elif isinstance(node, ast.UnaryOp):
            op_type = type(node.op)
            if op_type in self.UNARY_OPS:
                return self.UNARY_OPS[op_type](self.eval(node.operand))
            raise TypeError(f"Unsupported unary operator: {op_type}")

        elif isinstance(node, ast.BinOp):
            op_type = type(node.op)
            if op_type in self.BINARY_OPS:
                left = self.eval(node.left)
                right = self.eval(node.right)
                return self.BINARY_OPS[op_type](left, right)
            raise TypeError(f"Unsupported binary operator: {op_type}")

        elif isinstance(node, ast.Compare):
            left = self.eval(node.left)
            for op, comparator in zip(node.ops, node.comparators):
                op_type = type(op)
                if op_type not in self.COMP_OPS:
                    raise TypeError(f"Unsupported comparison operator: {op_type}")
                right = self.eval(comparator)
                if not self.COMP_OPS[op_type](left, right):
                    return False
                left = right
            return True

        elif isinstance(node, ast.Call):
            if isinstance(node.func, ast.Name):
                func_name = node.func.id
                args = [self.eval(arg) for arg in node.args]
                kwargs = {kw.arg: self.eval(kw.value) for kw in node.keywords}
                if func_name in self.context and callable(self.context[func_name]):
                    return self.context[func_name](*args, **kwargs)
                if func_name in self.ALLOWED_FUNCS:
                    return self.ALLOWED_FUNCS[func_name](*args, **kwargs)
                raise NameError(f"Function '{func_name}' is not allowed or defined")

            elif isinstance(node.func, ast.Attribute):
                obj = self.eval(node.func.value)
                method_name = node.func.attr
                if method_name in self.ALLOWED_METHODS and hasattr(obj, method_name):
                    args = [self.eval(arg) for arg in node.args]
                    kwargs = {kw.arg: self.eval(kw.value) for kw in node.keywords}
                    return getattr(obj, method_name)(*args, **kwargs)
                raise AttributeError(f"Method '{method_name}' is not allowed")

        elif isinstance(node, ast.Attribute):
            obj = self.eval(node.value)
            attr_name = node.attr
            if hasattr(obj, attr_name):
                return getattr(obj, attr_name)
            return None

        raise TypeError(f"Unsupported AST node type: {type(node).__name__}")


# ==========================================
# CORE MANAGER
# ==========================================
class RustatioManager:
    def __init__(self):
        self.api = APIClient(RUSTATIO_API)
        self.default_config = {}
        self.rules_lines = []
        self.strike_lock = threading.Lock()
        self.logs_state = self.load_logs_state()
        self.rand_cache = {}
        self.current_instances = []
        
        self.stop_event = threading.Event()
        self.watcher_stop_event = threading.Event()
        self.logs_thread = None

        log("Initializing regex patterns for RustatioManager", "trace")
        self.re_range = re.compile(r'([A-Za-z0-9_.]+): *([0-9.]+) *- *([0-9.]+)')
        self.re_default_config = re.compile(r'default_config\.([A-Za-z0-9_.]+)')
        self.re_num_cmp = re.compile(r'([A-Za-z0-9_.]+) *(<=|>=|<|>) *([0-9.]+)')
        self.re_num_eq = re.compile(r'([A-Za-z0-9_.]+) *(!=|=) *([0-9.]+)')
        self.re_num_neq = re.compile(r'([A-Za-z0-9_.]+) *!= *([0-9.]+)')
        self.re_bool_null_eq = re.compile(r'([A-Za-z0-9_.]+) *= *(true|false|null)', re.IGNORECASE)
        self.re_bool_null_colon = re.compile(r'([A-Za-z0-9_.]+) *: *(true|false|null)', re.IGNORECASE)
        self.re_inst_keys = re.compile(r'([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)+)')

        self.regex_subs = [
            (re.compile(r'torrent\.(announce|name|comment|created_by) *~ *"([^"]+)"'), r'("\2".lower() in str(__get__("torrent.\1", "")).lower())'),
            (re.compile(r'torrent\.(announce|name|comment|created_by) *~ *([A-Za-z0-9_@./:-]+)'), r'("\2".lower() in str(__get__("torrent.\1", "")).lower())'),
            (re.compile(r'tags *!= *"([^"]+)"'), r'("\1" not in __tags__)'),
            (re.compile(r'tags *!= *([A-Za-z0-9_@./:-]+)'), r'("\1" not in __tags__)'),
            (re.compile(r'tags *: *"([^"]+)"'), r'("\1" in __tags__)'),
            (re.compile(r'tags *: *([A-Za-z0-9_@./:-]+)'), r'("\1" in __tags__)'),
            (re.compile(r'tags *= *"([^"]+)"'), r'("\1" in __tags__)'),
            (re.compile(r'tags *= *([A-Za-z0-9_@./:-]+)'), r'("\1" in __tags__)'),
            (re.compile(r'torrent\.info_hash *: *"([A-Fa-f0-9]+)"'), r'(__info_hash__() == "\1")'),
            (re.compile(r'torrent\.info_hash *([A-Fa-f0-9]+)'), r'(__info_hash__() == "\1")'),
            (re.compile(r'torrent\.info_hash *= *"([A-Fa-f0-9]+)"'), r'(__info_hash__() == "\1")'),
            (re.compile(r'torrent\.info_hash *= *([A-Fa-f0-9]+)'), r'(__info_hash__() == "\1")'),
            (re.compile(r'([A-Za-z0-9_.]+) *!= *"([^"]+)"'), r'(str(__get__("\1", "")) != "\2")'),
            (re.compile(r'([A-Za-z0-9_.]+) *!= *([A-Za-z0-9_@./:-]+)'), r'(str(__get__("\1", "")) != "\2")'),
            (re.compile(r'([A-Za-z0-9_.]+) *= *"([^"]+)"'), r'(str(__get__("\1", "")) == "\2")'),
            (re.compile(r'([A-Za-z0-9_.]+) *= *([A-Za-z0-9_@./:-]+)'), r'(str(__get__("\1", "")) == "\2")'),
            (re.compile(r'([A-Za-z0-9_.]+) *: *"([^"]+)"'), r'(str(__get__("\1", "")) == "\2")'),
            (re.compile(r'([A-Za-z0-9_.]+) *: *([A-Za-z0-9_@./:-]+)'), r'(str(__get__("\1", "")) == "\2")')
        ]

    def get_val(self, data, path, default=None):
        current = data
        for part in path.split('.'):
            if isinstance(current, dict) and part in current:
                current = current.get(part)
            else:
                log(f"get_val path '{path}' missed at part '{part}', returning default: {default}", "trace")
                return default
        return current

    def get_num(self, data, path, default=0):
        val = self.get_val(data, path, default)
        try:
            return float(val) if val is not None else float(default)
        except (ValueError, TypeError):
            log(f"get_num conversion failed for path '{path}' with value '{val}', falling back to default: {default}", "trace")
            return float(default)

    def _extract_bracket(self, msg):
        start = msg.find("[")
        if start == -1: return None, msg
        level = 0
        for i in range(start, len(msg)):
            if msg[i] == "[": level += 1
            elif msg[i] == "]":
                level -= 1
                if level == 0: return msg[start+1:i].strip(), msg[i+1:].strip()
        return msg[start:], ""

    def load_configs(self):
        log("Loading configurations...", "trace")
        try:
            if os.path.exists(DEFAULTS_FILE):
                with open(DEFAULTS_FILE, 'r') as f:
                    data = json.load(f)
                    self.default_config = data.get("default_config", data)
                    log(f"Defaults loaded from {DEFAULTS_FILE}", "start")
                    log(f"Loaded default_config content: {self.default_config}", "trace")
            else:
                log(f"Defaults file {DEFAULTS_FILE} not found", "error")
        except Exception as e:
            self.default_config = {}
            log(f"Defaults file {DEFAULTS_FILE} is not valid JSON (Error: {e})", "error")

        try:
            if os.path.exists(RULES_FILE):
                with open(RULES_FILE, "r") as f:
                    raw_lines = [l.strip() for l in f if l.strip() and not l.startswith("#")]

                self.rules_lines = []
                for line in raw_lines:
                    log(f"Processing rule line: {line}", "trace")
                    parts = line.split('|', 2)
                    if len(parts) == 3:
                        cond, action, assign = [x.strip() for x in parts]
                        default_keys = set(self.re_default_config.findall(cond))
                        all_keys = set(self.re_inst_keys.findall(cond))
                        instance_keys = { k for k in all_keys if not k.startswith("default_config.") and not k.startswith("torrent.info_hash") }

                        py_cond = self.translate_condition(cond)
                        try:
                            ast_tree = ast.parse(py_cond, mode='eval')
                            self.rules_lines.append({
                                "raw": line, "cond_str": cond, "ast_tree": ast_tree,
                                "action": action, "assign": assign,
                                "default_keys": default_keys, "instance_keys": instance_keys
                            })
                            log(f"Rule successfully parsed to AST -> Condition: {py_cond}", "trace")
                        except SyntaxError as e:
                            log(f"Generated rule syntax error: {py_cond} ({e})", "error")
                    else:
                        log(f"Invalid rule skipped: {line}", "denied")

                log(f"Rules loaded and compiled to AST from {RULES_FILE}", "start")
            else:
                log(f"Rules file {RULES_FILE} not found", "error")
        except Exception as e:
            log(f"Error: {str(e)}", "error")

    def load_logs_state(self):
        log(f"Loading logs state from {CHECK_LOGS_FILE}", "trace")
        try:
            if os.path.exists(CHECK_LOGS_FILE):
                with open(CHECK_LOGS_FILE, 'r') as f:
                    return json.load(f)
        except Exception as e:
            log(f"Error: {str(e)}", "error")
        return {}

    def save_logs_state(self):
        log(f"Saving logs state to {CHECK_LOGS_FILE}", "trace")
        try:
            persisted_state = {
                tag: {k: v for k, v in state.items() if k != "announcing_time"}
                for tag, state in self.logs_state.items()
                if self._has_persistent_data(state)
            }
            with open(CHECK_LOGS_FILE, 'w') as f:
                json.dump(persisted_state, f)
        except Exception as e:
            log(f"Error: {str(e)}", "error")

    def format_time_bash_style(self, t):
        h = t // 3600
        m = (t % 3600) // 60
        if h > 0: return f"{h}h{m:02d}" if m > 0 else f"{h}h"
        return f"{m}min"

    def get_elapsed_since_midnight_tag(self, ts):
        t_struct = time.localtime(ts)
        midnight = time.mktime((t_struct.tm_year, t_struct.tm_mon, t_struct.tm_mday, 0, 0, 0, t_struct.tm_wday, t_struct.tm_yday, t_struct.tm_isdst))
        return self.format_time_bash_style(int(ts - midnight))

    def get_cached_rand(self, path, low, high, raw_match, tracker=None):
        key = f"{path}:{low}:{high}:{raw_match}"
        if key not in self.rand_cache:
            l, h = float(low), float(high)
            if l > h: l, h = h, l
            self.rand_cache[key] = random.uniform(l, h)
            log(f"Generated new cached random for {key}: {self.rand_cache[key]}", "trace")
        if tracker is not None:
            tracker.append(key)
        return self.rand_cache[key]

    def clear_rand_keys(self, keys):
        if keys:
            for k in keys:
                if k in self.rand_cache:
                    self.rand_cache.pop(k, None)
                    log(f"Purged cached random key after API success: {k}", "trace")

    def validate_rule_keys(self, rule, sample_inst):
        for key in rule["default_keys"]:
            if self.get_val(self.default_config, key, "__MISSING__") == "__MISSING__":
                log(f"default_config key 'default_config.{key}' not found", "f_error")
                return False
        for key in rule["instance_keys"]:
            if self.get_val(sample_inst, key, "__MISSING__") == "__MISSING__":
                log(f"Instance key '{key}' not found", "f_error")
                return False
        return True

    def translate_condition(self, cond_str):
        c = re.sub(r"\b(AND|OR)\b", lambda m: m.group(0).lower(), cond_str)
        c = self.re_default_config.sub(r'__default__("\1")', c)

        def repl_bool(m):
            val_map = {"true": "True", "false": "False", "null": "None"}
            return f"(__get__('{m.group(1)}') == {val_map[m.group(2).lower()]})"
        
        c = self.re_bool_null_eq.sub(repl_bool, c)
        c = self.re_bool_null_colon.sub(repl_bool, c)

        def repl_range(m):
            path, low_str, high_str = m.group(1), m.group(2), m.group(3)
            return f"(__get_num__('{path}') > __get_rand__('{path}', {low_str}, {high_str}, '{m.group(0)}'))"
        c = self.re_range.sub(repl_range, c)

        for pattern, repl in self.regex_subs:
            c = pattern.sub(repl, c)

        c = self.re_num_cmp.sub(r'(__get_num__("\1") \2 \3)', c)
        c = self.re_num_eq.sub(lambda m: f"(__get_num__('{m.group(1)}') {'==' if m.group(2) == '=' else '!='} {m.group(3)})", c)
        c = self.re_num_neq.sub(r'(__get_num__("\1") != \2)', c)
        return c

    def evaluate_rule(self, inst, ast_tree):
        eval_rand_keys = []
        eval_context = {
            '__get__': lambda path, d=None: self.get_val(inst, path, d),
            '__get_num__': lambda path: self.get_num(inst, path),
            '__tags__': inst.get("tags") or [],
            '__info_hash__': lambda: bytes(inst.get("torrent", {}).get("info_hash", [])).hex(),
            '__default__': lambda k: self.get_val(self.default_config, k),
            '__get_rand__': lambda path, low, high, raw_match: self.get_cached_rand(path, low, high, raw_match, eval_rand_keys),
            'str': str,
            'float': float,
            'int': int,
            'bool': bool,
            'True': True,
            'False': False,
            'None': None,
        }
        try:
            evaluator = ASTEvaluator(eval_context)
            result = evaluator.eval(ast_tree)
#            log(f"Evaluated rule for instance ID {inst.get('id')}: result = {result}", "trace")
            return bool(result), eval_rand_keys
        except Exception as e:
            log(f"Evaluation error for instance ID {inst.get('id')}: {e}", "trace")
            return False, []

    def is_action_valid(self, action, state):
        if action == "start": return (state == "Stopped")
        if action == "stop": return state in ("Running", "Stopping")
        if action == "pause": return (state == "Running")
        if action == "resume": return (state == "Paused")
        return action in ["update", "addtags", "removetags", "delete"]

    def _resolve_assignment_val(self, val_raw):
        val_raw = val_raw.strip()
        if val_raw.startswith("default_config."): return self.get_val(self.default_config, val_raw.replace("default_config.", ""))
        lower_val = val_raw.lower()
        if lower_val in ["true", "false", "null"]: return True if lower_val == "true" else False if lower_val == "false" else None
        if (val_raw.startswith('"') and val_raw.endswith('"')) or (val_raw.startswith("'") and val_raw.endswith("'")): return val_raw[1:-1]
        try: return float(val_raw) if "." in val_raw else int(val_raw)
        except ValueError: return val_raw

    def process_rules(self):
        log("Starting process_rules cycle", "trace")
        instances_resp = self.api.request("GET", "instances")
        if not instances_resp:
            log("process_rules aborted: no response from instances endpoint", "trace")
            return

        with self.strike_lock:
            self.current_instances = instances_resp.get("data", [])

        if not self.current_instances:
            log("process_rules: current_instances list is empty", "trace")
            return
        sample_inst = self.current_instances[0]

        for rule in self.rules_lines:
            if not self.validate_rule_keys(rule, sample_inst):
                log(f"Rule keys validation failed for rule: {rule['raw']}", "trace")
                continue
            for inst in self.current_instances:
                if inst.get("_deleted"): continue
                state = self.get_val(inst, "stats.state")
                if not self.is_action_valid(rule["action"], state): continue
                if TOR_KEEP_LAST and rule["action"] in ["stop", "delete"]:
                    announce = str(self.get_val(inst, "torrent.announce", "")).lower()
                    count = sum(1 for i in self.current_instances if not i.get("_deleted") and str(self.get_val(i, "torrent.announce", "")).lower() == announce)
                    if count <= 1:
                        log(f"TOR_KEEP_LAST triggered: skipping action '{rule['action']}' for instance {inst.get('id')}.", "trace")
                        continue

                matched, rand_keys = self.evaluate_rule(inst, rule["ast_tree"])
                if matched:
                    log(f"Rule matched! Applying action '{rule['action']}' on instance ID {inst.get('id')}", "trace")   
                    self.apply_action(inst, rule["action"], rule["assign"], rule["raw"], rand_keys)

    def apply_action(self, inst, action, assign, rule_line, used_keys=None):
        id_ = inst.get("id")
        name = self.get_val(inst, "torrent.name")

        log(f"Rule applied: {rule_line}", "task")
        log(f"Torrent name : {name}", "f_data")

        if action == "addtags":
            tags_to_add = [t.strip() for t in assign.split(',') if t.strip()]
            existing_tags = inst.get("tags") or []
            new_tags = [t for t in tags_to_add if t not in existing_tags]

            if not new_tags: return
            payload = {"ids": [id_], "add_tags": new_tags, "remove_tags": []}
            if DRY_RUN: log(f"Would add tags to ID='{id_}' TAGS='{payload}'", "f_recycle")
            elif self.api.request("POST", "grid/tag", payload):
                inst["tags"] = list(set(existing_tags + new_tags))
                log(f"Tags added ({assign})", "f_succes")
                self.clear_rand_keys(used_keys)
            else:
                log(f"Failed to add tags ({assign}) for instance {id_}", "f_error")

        elif action == "removetags":
            tags_to_remove = [t.strip() for t in assign.split(',') if t.strip()]
            existing_tags = inst.get("tags") or []
            del_tags = [t for t in tags_to_remove if t in existing_tags]

            if not del_tags: return
            payload = {"ids": [id_], "add_tags": [], "remove_tags": del_tags}
            if DRY_RUN: log(f"Would remove tags from ID='{id_}' TAGS='{payload}'", "f_recycle")
            elif self.api.request("POST", "grid/tag", payload):
                inst["tags"] = [t for t in existing_tags if t not in del_tags]
                log(f"Tags removed ({assign})", "f_succes")
                self.clear_rand_keys(used_keys)
            else:
                log(f"Failed to remove tags ({assign}) for instance {id_}", "f_error")

        elif action == "update":
            parts = assign.split("=", 1)
            if len(parts) == 2 and parts[0].strip().startswith("config."):
                key = parts[0].replace("config.", "").strip()
                val = self._resolve_assignment_val(parts[1].strip().rstrip(";"))
                if val == "__MISSING__":
                    log(f"default_config key '{parts[1].strip()}' not found in defaults", "f_error")
                    return

                payload = copy.deepcopy(inst.get("config", {}))
                payload[key] = val

                if DRY_RUN: log(f"Would patch config ID='{id_}' PAYLOAD='{payload}'", "f_recycle")
                elif self.api.request("PATCH", f"instances/{id_}/config", payload):
                    inst.setdefault("config", {})[key] = val
                    log(f"Patch succeeded ({assign})", "f_succes")
                    self.clear_rand_keys(used_keys)
                else: log("Patch failed", "f_error")
            else: log(f"update: invalid assign '{assign}'", "warning")

        elif action == "start":
            payload = {"torrent": copy.deepcopy(inst.get("torrent", {})), "config": copy.deepcopy(inst.get("config", {}))}
            if DRY_RUN: log(f"Would start ID='{id_}'", "f_recycle")
            elif self.api.request("POST", f"faker/{id_}/start", payload):
                inst.setdefault("stats", {})["state"] = "Running"
                log("Start succeeded", "f_succes")
                self.clear_rand_keys(used_keys)

        elif action in ["stop", "pause", "resume"]:
            endpoints = {"stop": f"faker/{id_}/stop", "pause": "grid/pause", "resume": "grid/resume"}
            payload = {"ids": [id_]} if action in ["pause", "resume"] else None
            if DRY_RUN: log(f"Would {action} ID='{id_}'", "f_recycle")
            elif self.api.request("POST", endpoints[action], payload):
                inst.setdefault("stats", {})["state"] = "Stopped" if action == "stop" else ("Paused" if action == "pause" else "Running")
                log(f"{action.capitalize()} succeeded", "f_succes")
                self.clear_rand_keys(used_keys)

        elif action == "delete":
            if "instance" in assign:
                if DRY_RUN: log(f"Would delete ID='{id_}'", "f_recycle")
                elif self.api.request("DELETE", f"instances/{id_}?force=true"):
                    inst["_deleted"] = True
                    log("Delete succeeded", "f_succes")
                    self.clear_rand_keys(used_keys)

            if "watchfile" in assign or "archive" in assign:
                hex_hash = bytes(inst.get("torrent", {}).get("info_hash", [])).hex()
                if hex_hash:
                    files_resp = self.api.request("GET", "watch/files")
                    files = files_resp.get("data", []) if files_resp else []                                                
                    for f in files:
                        if f.get("info_hash") == hex_hash:
                            filename, filepath = f.get("filename", ""), f.get("path", "")
                            
                            if "archive" in assign and ARCHIVE_FOLDER:
                                dest = os.path.join(ARCHIVE_FOLDER, os.path.basename(filename))
                                if not os.path.exists(dest):
                                    if DRY_RUN: log(f"Would archive file '{filename}' at '{filepath}'", "f_recycle")
                                    else:
                                        os.makedirs(ARCHIVE_FOLDER, exist_ok=True)
                                        try:
                                            shutil.copy2(filepath, dest)
                                            log(f"Torrent archived {dest}", "f_saving")
                                        except Exception: log(f"Failed to archive {filepath}", "f_error")

                            if "watchfile" in assign:
                                if DRY_RUN: log(f"Would delete file '{filename}' at '{filepath}'", "f_recycle")
                                else:
                                    if self.api.request("DELETE", f"watch/files?path={urllib.parse.quote(filename)}"):
                                        log("Delete succeeded", "f_succes")
                                    else:
                                        log("Failed to delete file", "f_error")

    # ------------------------------------------------------------------
    # Helpers partagés
    # ------------------------------------------------------------------
    def _find_instance_by_name(self, name):
        with self.strike_lock:
            for inst in self.current_instances:
                if self.get_val(inst, "torrent.name") == name: return inst
        return None

    def _should_stop(self):
        return self.stop_event.is_set() or self.watcher_stop_event.is_set()

    def _auth_headers(self):
        return {"Authorization": f"Bearer {AUTH_TOKEN}"} if AUTH_TOKEN else {}

    def _err_tag(self, ts):
        return f"Err {self.get_elapsed_since_midnight_tag(ts)}"

    @staticmethod
    def _new_tag_state():
        return {"counts": {}, "action": 0, "last_count_time": 0, "announcing_time": 0}

    @staticmethod
    def _has_persistent_data(state):
        """True si l'état serait écrit par save_logs_state (donc si sa suppression change le fichier)."""
        return state.get("action", 0) > 0 or bool(state.get("counts"))

    def _inst_tags(self, inst):
        """Tags d'une instance sous forme de liste (gère None et chaîne 'a, b')."""
        tags = inst.get("tags") or []
        if isinstance(tags, str):
            tags = [t.strip() for t in tags.split(',') if t.strip()]
        return tags

    def _update_tags(self, inst, add=(), remove=()):
        """Ajoute/retire des tags via l'API et met à jour l'instance locale.
        Retourne True uniquement si des tags ont réellement été modifiés."""
        existing = self._inst_tags(inst)
        add = [t for t in add if t not in existing]
        remove = [t for t in remove if t in existing]
        if not add and not remove:
            return False
        payload = {"ids": [inst.get("id")], "add_tags": add, "remove_tags": remove}
        if not self.api.request("POST", "grid/tag", payload):
            return False
        inst["tags"] = [t for t in existing if t not in remove] + add
        return True

    def _grid_action(self, inst, action):
        """pause / resume via l'API + mise à jour de l'état local."""
        new_state = {"pause": "Paused", "resume": "Running"}[action]
        if not self.api.request("POST", f"grid/{action}", {"ids": [inst.get("id")]}):
            return False
        inst.setdefault("stats", {})["state"] = new_state
        return True

    # ------------------------------------------------------------------
    # Thread SSE
    # ------------------------------------------------------------------
    def logs_watcher_thread(self):
        url = f"{self.api.base_url}/api/logs"
        log(f"Starting logs_watcher_thread targeting SSE URL: {url}", "trace")
        headers = {
            "Accept": "text/event-stream",
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",
            **self._auth_headers(),
        }

        while not self._should_stop():
            try:
                with requests.get(url, stream=True, headers=headers, timeout=60) as r:
                    r.encoding = "utf-8"
                    for line in r.iter_lines(decode_unicode=True):
                        if self._should_stop():
                            return
                        self._check_expirations()
                        if line and line.startswith("data:"):
                            self._handle_log_event(line[5:].strip())
            except requests.exceptions.Timeout:
                self._check_expirations()
            except Exception as e:
                if self._should_stop():
                    log("logs_watcher_thread stopping", "trace")
                    return
                log(f"logs_watcher_thread exception encountered: {e}", "trace")
                self.stop_event.wait(5)

    # ------------------------------------------------------------------
    # Traitement d'un évènement de log
    # ------------------------------------------------------------------
    def _handle_log_event(self, json_str):
        log(f"Handling log event payload: {json_str}", "trace")
        try:
            event = json.loads(json_str)
            msg = event.get("message", "")
            if "[" not in msg:
                return
            tag, rest = self._extract_bracket(msg)
            if not tag:
                return

            level = event.get("level", "").lower()
            now = time.time()

            inst = self._find_instance_by_name(tag)
            if inst:
                inst_state = self.get_val(inst, "stats.state")
                if inst_state in ("Paused", "Stopped"):
                    log(f"Log ignored for '{tag}': torrent status is {inst_state}.", "trace")
                    return

            with self.strike_lock:
                state = self.logs_state.get(tag, {})
                if state and state.get("action", 0) > 0:
                    log(f"Log ignored for '{tag}': watcher pause is currently active.", "trace")
                    return
                state = self.logs_state.setdefault(tag, self._new_tag_state())
                should_pause = self._register_event(tag, state, level, msg, rest, now)

            if should_pause:
                self._watcher_apply(tag, "pause", now, detail=rest)

        except Exception as e:
            log(f"Error: {str(e)}", "error")

    def _is_announce_active(self, state, now):
        has_info_log = get_global_rust_log_level(os.getenv("RUST_LOG", "info")) <= logging.INFO
        if not has_info_log:
            return True
        announce_ts = state.get("announcing_time", 0)
        return announce_ts > 0 and (now - announce_ts) <= WATCHER_ANNOUNCE_TIMEOUT

    def _register_event(self, tag, state, level, msg, rest, now):
        """Met à jour l'état d'un tag. À appeler sous strike_lock.
        Retourne True si une pause doit être déclenchée."""
        low = msg.lower()
        is_info = "info" in level

        if is_info and "announcing" in low:
            state["announcing_time"] = now
            log(f"Announce detected for '{tag}', marking announce as active.", "trace")
            return False

        if is_info and ("complete" in low or "successfully" in low):
            del self.logs_state[tag]
            log(f"Complete detected for '{tag}', cleanup.", "trace")
            return False

        if "warn" not in level:
            return False

        if not self._is_announce_active(state, now):
            log(f"Error ignored for '{tag}' because no active announce in progress.", "trace")
            return False

        key = re.sub(r'https?://\S+', '<url>', rest)
        counts = state.setdefault("counts", {})
        counts[key] = counts.get(key, 0) + 1
        count = counts[key]
        state["last_count_time"] = now
        state["announcing_time"] = 0
        log(f"Error during active announce for '{tag}' (strike {count}/{WATCHER_MAX_STRIKE})", "trace")

        should_pause = state["action"] == 0 and count >= WATCHER_MAX_STRIKE
        if should_pause:
            state["action"] = now
        self.save_logs_state()
        return should_pause

    # ------------------------------------------------------------------
    # Pause / reprise
    # ------------------------------------------------------------------
    def _watcher_apply(self, tag, action, action_ts, detail=None):
        """action = 'pause' (+ ajout du tag Err) ou 'resume' (- retrait du tag Err)."""
        inst = self._find_instance_by_name(tag)
        if not inst:
            return
        if not self.is_action_valid(action, self.get_val(inst, "stats.state")):
            return

        pausing = action == "pause"
        err_tag = self._err_tag(action_ts)

        if pausing:
            log(f"Repeated error detected (x{WATCHER_MAX_STRIKE}). Try to pause for "
                f"{self.format_time_bash_style(WATCHER_PAUSE_TIME)} and add tag", "watcher")
        else:
            log("Pause ended. Try to resume and remove tag", "watcher")
        log(f"Torrent name : {tag}", "f_marked")
        if detail:
            log(f"{detail}", "f_data")

        if DRY_RUN:
            tag_op = "addtags" if pausing else "removetags"
            log(f"Would {action} '{tag}' and {tag_op} '{err_tag}'", "f_recycle")
            return

        if not self._grid_action(inst, action):
            return
        log(f"{action.capitalize()} succeeded", "f_succes")

        if pausing:
            changed = self._update_tags(inst, add=[err_tag])
        else:
            changed = self._update_tags(inst, remove=[err_tag])
        if changed:
            log(f"Tags {'applied' if pausing else 'removed'} ({err_tag})", "f_succes")

    # ------------------------------------------------------------------
    # Expirations
    # ------------------------------------------------------------------
    def _expire_state(self, tag, state, now, is_active):
        """Applique les expirations sur l'état d'un tag. À appeler sous strike_lock.
        Retourne (modifié, action_ts_à_reprendre_ou_None, à_supprimer)."""
        if not is_active:
            announce_ts = state.get("announcing_time", 0)
            if announce_ts > 0 and (now - announce_ts) <= WATCHER_ANNOUNCE_TIMEOUT:
                return False, None, False
            return False, None, True

        changed = False

        announce_ts = state.get("announcing_time", 0)
        if announce_ts > 0 and (now - announce_ts) > WATCHER_ANNOUNCE_TIMEOUT:
            state["announcing_time"] = 0
            changed = False
            log(f"Announce status expired for '{tag}'", "trace")

        action_ts = state.get("action", 0)
        last_count = state.get("last_count_time", 0)

        if action_ts == 0 and last_count > 0 and (now - last_count) > WATCHER_STRIKE_TIME:
            state["counts"] = {}
            state["last_count_time"] = 0
            changed = True

        if action_ts > 0 and (now - action_ts) > WATCHER_PAUSE_TIME:
            return changed, action_ts, True

        is_empty = (
            action_ts == 0
            and not state.get("announcing_time", 0)
            and not any(c > 0 for c in state.get("counts", {}).values())
        )
        return changed, None, is_empty

    def _check_expirations(self):
#        log("Checking log expirations and purges", "trace")
        to_resume = []
        dirty = False

        with self.strike_lock:
            if not self.current_instances:
                log("Skip _check_expirations: current_instances is empty", "trace")
                return

            now = time.time()
            active_names = {self.get_val(i, "torrent.name") for i in self.current_instances}

            for tag, state in list(self.logs_state.items()):
                had_data = self._has_persistent_data(state)
                changed, resume_ts, remove = self._expire_state(tag, state, now, tag in active_names)
                dirty |= changed
                if resume_ts:
                    to_resume.append((tag, resume_ts))
                if remove:
                    del self.logs_state[tag]
                    dirty |= had_data

        if dirty:
            self.save_logs_state()

        for tag, action_ts in to_resume:
            self._watcher_apply(tag, "resume", action_ts)

    # ------------------------------------------------------------------
    # Reset manuel
    # ------------------------------------------------------------------
    def clear_strike_and_resume(self, tag):
        with self.strike_lock:
            if self.logs_state.pop(tag, None) is not None:
                self.save_logs_state()
                log(f"Strikes manually cleared for tag : {tag}", "warn")

        inst = self._find_instance_by_name(tag)
        if not inst:
            return False
        id_ = inst.get("id")

        err_tags = [t for t in self._inst_tags(inst) if t.startswith("Err ")]
        if err_tags and self._update_tags(inst, remove=err_tags):
            log(f"Tags deleted ({', '.join(err_tags)}) for instance ID : {id_}", "f_succes")

        state = self.get_val(inst, "stats.state")
        if not self.is_action_valid("resume", state):
            log(f"Resume skipped for instance ID : {id_} (state: {state})", "trace")
            return True

        if self._grid_action(inst, "resume"):
            log(f"Resume sent for instance ID : {id_}", "f_succes")
            return True
        return False

    def run(self):
        initial_interval = 10
        log(f"Starting RustatioManager daemon", "start")
        self.load_configs()
        self.stop_event.clear()

        interval = REFRESH_INTERVAL
        if interval == 0 and self.default_config:
            interval = int(self.get_val(self.default_config, "scrape_interval", 60))

        interval = max(0, interval - initial_interval)
        log(f"REFRESH INTERVAL : {interval + initial_interval}s", "data")

        if LOGS_WATCHER:
            self.start_watcher()

        headers = self._auth_headers()

        while not self.stop_event.is_set():
            try:
                resp = requests.get(f"{RUSTATIO_API}/health", headers=headers, timeout=3)
                if resp.text == "OK": break
            except Exception: pass
            if self.stop_event.is_set(): return
            log(f"Rustatio not ready. Retry in {initial_interval}s", "warning")
            self.stop_event.wait(initial_interval)

        while not self.stop_event.is_set():
            if self.stop_event.wait(initial_interval): break
            if LOGFILE and LOGFILE != "/dev/null" and not os.path.exists(LOGFILE):
                setup_file_handler()
                log("Log recreated automatically", "start")
                self.load_configs()

            try: 
                self.process_rules()
                if LOGS_WATCHER:
                    self._check_expirations()
            except Exception as e: 
                log(f"Error in process_rules: {str(e)}", "error")
            
            if self.stop_event.wait(interval): break
            
        log("RustatioManager stopped", "finish")

    def is_watcher_running(self):
        return self.logs_thread is not None and self.logs_thread.is_alive()

    def start_watcher(self):
        if not self.is_watcher_running():
            self.watcher_stop_event.clear()
            self.logs_thread = threading.Thread(target=self.logs_watcher_thread, daemon=True)
            self.logs_thread.start()
            log("Log Watcher started", "start")

    def stop_watcher(self):
        if self.is_watcher_running():
            self.watcher_stop_event.set()
            if self.logs_thread:
                self.logs_thread.join(timeout=3)
            log("Log Watcher stopped", "finish")

    def stop(self):
        self.stop_event.set()
        for h in logger.handlers:
            h.flush()

# ==========================================
# FLASK WEB PANEL (ADMIN)
# ==========================================
manager = None
manager_thread = None
request_history = defaultdict(list)
rate_limit_lock = threading.Lock()

def get_client_ip():
    x_forwarded = request.headers.get('X-Forwarded-For')
    if x_forwarded:
        return x_forwarded.split(',')[0].strip()
    return request.remote_addr or '127.0.0.1'

def rate_limit(max_requests=5, window_seconds=60, methods=None):
    def decorator(f):
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            if methods and request.method not in methods:
                return f(*args, **kwargs)

            ip = get_client_ip()
            now = time.time()
            
            with rate_limit_lock:
                request_history[ip] = [t for t in request_history[ip] if now - t < window_seconds]
                
                if len(request_history[ip]) >= max_requests:
                    if request.path.startswith('/api/'):
                        return jsonify({"error": "Rate limit dépassé. Réessayez plus tard."}), 429
                    return render_template_string(LOGIN_TEMPLATE, error=True), 429

                request_history[ip].append(now)

            return f(*args, **kwargs)
        return wrapper
    return decorator

@app.before_request
def require_auth():
    if not AUTH_TOKEN:
        return None
    if request.path == '/login':
        return None

    auth_header = request.headers.get("Authorization", "")
    token_cookie = request.cookies.get("auth_token", "")
    
    candidate = ""
    if auth_header.startswith("Bearer "):
        candidate = auth_header[7:].strip()
    elif token_cookie:
        candidate = token_cookie

    if candidate and hmac.compare_digest(candidate, AUTH_TOKEN):
        return None

    if request.path.startswith('/api/'):
        return jsonify({"error": "Accès non autorisé"}), 401

    return render_template_string(LOGIN_TEMPLATE, error=False), 401

@app.route('/login', methods=['POST'])
@rate_limit(max_requests=5, window_seconds=60)
def login():
    token = request.form.get('token', '')
    if AUTH_TOKEN and token == AUTH_TOKEN:
        resp = make_response(redirect('/'))
        resp.set_cookie('auth_token', token, httponly=True, samesite='Lax')
        return resp
    return render_template_string(LOGIN_TEMPLATE, error=True), 401

@app.route('/')
def index():
    rules_filename = os.path.basename(RULES_FILE) if RULES_FILE else "rules.txt"
    logfile_filename = os.path.basename(LOGFILE) if LOGFILE else "rustatio_daemon.log"

    token_param = request.args.get("token", "")
    if token_param and token_param == AUTH_TOKEN:
        resp = make_response(redirect('/'))
        resp.set_cookie('auth_token', token_param, httponly=True, samesite='Lax')
        return resp

    return render_template_string(
        HTML_TEMPLATE, 
        rules_filename=rules_filename, 
        logfile_filename=logfile_filename
    )

@app.route('/favicon.ico')
def favicon():
    response = make_response(FAVICON_SVG)
    response.headers['Content-Type'] = 'image/svg+xml'
    return response

@app.route('/api/status', methods=['GET'])
def get_status():
    global manager_thread
    if manager_thread and manager_thread.is_alive():
        return jsonify({"running": True, "pid": os.getpid()})
    return jsonify({"running": False})

daemon_lock = threading.Lock()

@app.route('/api/daemon/<action>', methods=['POST'])
@rate_limit(max_requests=2, window_seconds=60)
def manage_daemon(action):
    global manager, manager_thread
    
    with daemon_lock:
        if action == 'stop':
            if manager and manager_thread and manager_thread.is_alive():
                manager.stop()
                manager_thread.join(timeout=5)
            return jsonify({"success": True})

        elif action == 'start':
            if not manager_thread or not manager_thread.is_alive():
                manager = RustatioManager()
                manager_thread = threading.Thread(target=manager.run, daemon=True)
                manager_thread.start()
            return jsonify({"success": True})

        elif action == 'restart':
            if manager and manager_thread and manager_thread.is_alive():
                manager.stop()
                manager_thread.join(timeout=5)
            manager = RustatioManager()
            manager_thread = threading.Thread(target=manager.run, daemon=True)
            manager_thread.start()
            return jsonify({"success": True})

        return jsonify({"success": False, "error": "Action invalide ou état incorrect"})

@app.route('/api/rules', methods=['GET', 'POST'])
@rate_limit(max_requests=3, window_seconds=60, methods=['POST'])
def manage_rules():
    if request.method == 'POST':
        content = request.json.get('content', '')
        os.makedirs(os.path.dirname(RULES_FILE), exist_ok=True)
        with open(RULES_FILE, 'w', encoding='utf-8') as f:
            f.write(content)
        if manager:
            manager.load_configs()
        return jsonify({"success": True})
    else:
        content = ""
        if os.path.exists(RULES_FILE):
            with open(RULES_FILE, 'r', encoding='utf-8') as f:
                content = f.read()
        return jsonify({"content": content})

@app.route('/api/logs', methods=['GET'])
def get_logs():
    content = "Fichier log introuvable."
    try:
        lines_limit = int(request.args.get('lines', 100))
    except ValueError:
        lines_limit = 100

    if os.path.exists(LOGFILE):
        with open(LOGFILE, 'r', encoding='utf-8') as f:
            lines = f.readlines()
            content = "".join(lines[-lines_limit:])
    return jsonify({"content": content})

@app.route('/api/logs/clear', methods=['POST'])
@rate_limit(max_requests=5, window_seconds=60)
def clear_logs():
    if os.path.exists(LOGFILE):
        with open(LOGFILE, 'w', encoding='utf-8') as f:
            f.write("")
    return jsonify({"success": True})

@app.route('/api/logs/archive', methods=['POST'])
@rate_limit(max_requests=2, window_seconds=60)
def archive_logs():
    if not os.path.exists(LOGFILE):
        return jsonify({"success": False, "error": "Fichier de log introuvable"}), 404
    try:
        timestamp = datetime.datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        dir_name = os.path.dirname(LOGFILE)
        base_name = os.path.basename(LOGFILE)
        name, ext = os.path.splitext(base_name)
        archive_name = f"{name}_{timestamp}{ext}"
        archive_path = os.path.join(dir_name, archive_name)
        
        shutil.copy2(LOGFILE, archive_path)
        with open(LOGFILE, 'w', encoding='utf-8') as f:
            f.write(f"--- Logs archivés le {timestamp} ---\n")
            
        return jsonify({"success": True, "filename": archive_name})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500

@app.route('/api/admin/restart', methods=['POST'])
@rate_limit(max_requests=3, window_seconds=60)
def restart_admin():
    def delayed_restart():
        time.sleep(1)
        os._exit(42)

    threading.Thread(target=delayed_restart).start()
    return jsonify({"success": True, "message": "Redémarrage du processus en cours..."})

@app.route('/api/watcher/status', methods=['GET'])
def get_watcher_status():
    if manager:
        if manager.is_watcher_running():
            return jsonify({
                "running": True, 
                "status": "ok", 
                "pid": os.getpid()
            })
        elif getattr(manager, 'watcher_error', None):
            return jsonify({
                "running": False, 
                "status": "crashed", 
                "error": manager.watcher_error
            })
    return jsonify({"running": False, "status": "stopped"})

@app.route('/api/watcher/state', methods=['GET'])
def get_watcher_state():
    if not manager:
        return jsonify({"state": {}})
    
    now = time.time()
    res = {}
    
    with manager.strike_lock:
        for tag, state in manager.logs_state.items():
            counts = sum(state.get("counts", {}).values())
            action_ts = state.get("action", 0)
            last_ts = state.get("last_count_time", 0)
            announce_ts = state.get("announcing_time", 0)
            
            if action_ts > 0:
                status = "paused"
                time_left = max(0, int((action_ts + WATCHER_PAUSE_TIME) - now))
            elif announce_ts > 0 and action_ts == 0:
                status = "announce"
                time_left = max(0, int((announce_ts + WATCHER_ANNOUNCE_TIMEOUT) - now))
            else:
                status = "observing"
                if last_ts > 0:
                    time_left = max(0, int((last_ts + WATCHER_STRIKE_TIME) - now))
                else:
                    time_left = 0
                
            res[tag] = {
                "errors": state.get("counts", {}),
                "total_strikes": counts,
                "status": status,
                "time_left": time_left,
                "announce_ts": announce_ts
            }
            
    return jsonify({"state": res})

@app.route('/api/watcher/<action>', methods=['POST'])
@rate_limit(max_requests=3, window_seconds=60)
def manage_watcher(action):
    if not manager or not manager_thread or not manager_thread.is_alive():
        return jsonify({"success": False, "error": "Le daemon principal est arrêté"})

    if action == 'start':
        manager.start_watcher()
        return jsonify({"success": True})
    elif action == 'stop':
        manager.stop_watcher()
        return jsonify({"success": True})
    elif action == 'restart':
        manager.stop_watcher()
        time.sleep(3)
        manager.start_watcher()
        return jsonify({"success": True})

    return jsonify({"success": False, "error": "Action invalide"})

@app.route('/api/watcher/clear_resume', methods=['POST'])
@rate_limit(max_requests=25, window_seconds=60)
def api_watcher_clear_resume():
    data = request.json or {}
    tag = data.get("tag")
    if tag:
        success = manager.clear_strike_and_resume(tag)
        return jsonify({"success": success})
    return jsonify({"success": False, "error": "Tag manquant"}), 400

@app.route('/api/env', methods=['GET', 'POST'])
@rate_limit(max_requests=5, window_seconds=60, methods=['POST'])
def handle_env_config():
    if request.method == 'POST':
        success, message = config.update(request.json or {})
        if not success:
            return jsonify({"status": "forbidden", "message": message}), 403
        return jsonify({"status": "success", "message": message})

    return jsonify({
        "config": config.state,
        "readonly": list(config.readonly_keys)
    })

@app.route('/api/env/reset', methods=['POST'])
@rate_limit(max_requests=5, window_seconds=60)
def reset_env_config():
    config.reset()
    return jsonify({"status": "success", "message": "Configuration réinitialisée aux valeurs d'origine"})

def run_flask_app():
    log_werkzeug = logging.getLogger('werkzeug')
    log_werkzeug.setLevel(logging.ERROR)
    
    log(f"Admin web panel started on port {ADMIN_PORT}", "start")
    app.run(host='0.0.0.0', port=ADMIN_PORT, debug=False, use_reloader=False)

if __name__ == "__main__":
    ensure_paths_exist()

    manager = RustatioManager()
    manager_thread = threading.Thread(target=manager.run, daemon=True)
    manager_thread.start()

    run_flask_app()
