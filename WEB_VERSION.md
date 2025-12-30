# 🌐 Using Rustatio Web Version

Welcome to the browser-based version of Rustatio! This guide will help you get started and understand why you need a CORS proxy.

## 🤔 What is CORS and Why Do I Need a Proxy?

### The Problem

When you use Rustatio in your web browser, it needs to communicate with BitTorrent trackers to announce your fake seeding activity. However, web browsers have a security feature called **CORS (Cross-Origin Resource Sharing)** that prevents websites from making requests to other domains unless those domains explicitly allow it.

**The issue:** BitTorrent trackers were never designed to work with web browsers, so they don't send the special CORS headers that browsers require. This means your browser will block all requests to trackers, and Rustatio won't work.

```
Your Browser → ❌ Tracker (blocked by CORS)
```

### The Solution: CORS Proxy

A CORS proxy is a simple middleman service that:
1. Receives requests from your browser
2. Forwards them to the tracker
3. Adds the required CORS headers to the response
4. Sends the response back to your browser

```
Your Browser → ✅ CORS Proxy → Tracker
              (adds CORS headers)
```

**Good news:** Setting up a free CORS proxy takes just 5 minutes!

## 🚀 Quick Start Guide

### Option 1: Use Cloudflare Workers (Recommended - Free Forever)

Cloudflare Workers is a free service that lets you run code at the edge. Perfect for a CORS proxy!

**Why Cloudflare Workers?**
- ✅ **Free tier**: 100,000 requests per day (more than enough)
- ✅ **Fast**: Runs on Cloudflare's global network
- ✅ **Private**: You control it, only you use it
- ✅ **Reliable**: 99.99% uptime
- ✅ **No credit card required** for free tier

#### Step 1: Create a Cloudflare Account

1. Go to https://dash.cloudflare.com/sign-up
2. Sign up with your email (free account, no credit card needed)
3. Verify your email

#### Step 2: Create Your CORS Proxy Worker

1. Go to https://dash.cloudflare.com
2. Click **Workers & Pages** in the left sidebar
3. Click **Create Application**
4. Click **Create Worker**
5. Click **Deploy** (you can change the name if you want)
6. Click **Edit Code**
7. **Delete all the existing code** and paste this:

```javascript
export default {
  async fetch(request) {
    // Handle preflight OPTIONS request
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: {
          'Access-Control-Allow-Origin': '*',
          'Access-Control-Allow-Methods': 'GET, HEAD, OPTIONS',
          'Access-Control-Allow-Headers': '*',
          'Access-Control-Max-Age': '86400',
        }
      });
    }

    // Get target URL from query parameter
    const url = new URL(request.url);
    const targetUrl = url.searchParams.get('url');
    
    if (!targetUrl) {
      return new Response('Missing url parameter', { 
        status: 400,
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    }

    try {
      // Fetch from tracker
      const response = await fetch(targetUrl, {
        method: 'GET',
        headers: {
          'User-Agent': 'RustatioWeb/1.0'
        }
      });
      
      // Create new response with CORS headers
      const newResponse = new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
      });
      
      newResponse.headers.set('Access-Control-Allow-Origin', '*');
      newResponse.headers.set('Access-Control-Allow-Methods', 'GET, HEAD, OPTIONS');
      
      return newResponse;
    } catch (error) {
      return new Response('Proxy error: ' + error.message, {
        status: 502,
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    }
  }
}
```

8. Click **Save and Deploy**
9. Copy your worker URL (it looks like `https://your-worker-name.your-username.workers.dev`)

#### Step 3: Configure Rustatio to Use Your Proxy

1. Open Rustatio web version
2. Look for the **🌐 CORS Proxy (Optional)** section in Settings
3. Paste your worker URL in the **Proxy URL** field
4. Click **💾 Save Proxy**
5. Reload the page

**That's it! You're ready to use Rustatio! 🎉**

### Option 2: Desktop App (No Proxy Needed)

If you don't want to set up a CORS proxy, consider using the **desktop version** of Rustatio instead:

- ✅ No CORS issues (doesn't run in a browser)
- ✅ No proxy setup needed
- ✅ Slightly better performance
- ✅ Works offline

**Download:** [Releases Page](https://github.com/yourusername/rustatio/releases)

## ❓ Frequently Asked Questions

### Can I use the web version without a proxy?

**No.** Without a CORS proxy, the browser will block all tracker requests and Rustatio won't function. You have two options:
1. Set up a free CORS proxy (5 minutes)
2. Use the desktop app instead (no proxy needed)

### Is my data private when using a proxy?

**Yes, if you use your own Cloudflare Worker.** The proxy you create is:
- ✅ Only accessible to you
- ✅ Runs on your Cloudflare account
- ✅ No third parties involved

The proxy only sees the same information that the tracker sees anyway (torrent info hash, your fake upload/download stats).

### Does the proxy cost money?

**No!** Cloudflare Workers has a very generous free tier:
- **100,000 requests per day** (free forever)
- Typical Rustatio usage: ~300-500 requests per day
- You won't hit the limit unless you're running dozens of torrents 24/7

### Can others use my proxy?

**No.** Your proxy URL is unique to you. Nobody else knows it unless you share it.

If you want to keep it extra private, you can:
1. Choose a random worker name (e.g., `my-secret-proxy-abc123`)
2. Don't share the URL with anyone

### What if my proxy stops working?

Cloudflare Workers are extremely reliable (99.99% uptime). If you ever have issues:

1. **Test the proxy:** Open `https://your-worker.workers.dev/?url=https://www.google.com` in your browser
   - Should show Google's homepage
   - If it shows an error, check your worker code

2. **Check Cloudflare status:** https://www.cloudflarestatus.com/

3. **Redeploy:** You can always delete and recreate the worker

### Can I use someone else's proxy?

**Not recommended.** Using someone else's CORS proxy means:
- ❌ They can see all your tracker requests
- ❌ They might rate limit or block you
- ❌ They might shut it down at any time

Setting up your own takes 5 minutes and is free. It's worth it!

### Does this work on mobile?

**Yes!** The web version works on modern mobile browsers:
- ✅ iOS Safari (iOS 11+)
- ✅ Android Chrome (Chrome 57+)
- ✅ Firefox Mobile

You'll still need to configure a CORS proxy using the same steps above.

### Why not just fix CORS in the tracker?

BitTorrent trackers are designed for direct peer-to-peer communication, not web browsers. They have no reason to support CORS, and most tracker operators won't add it even if you ask.

This is why all browser-based BitTorrent tools (like WebTorrent) face the same CORS limitations.

## 🆘 Troubleshooting

### "CORS request did not succeed"

**Cause:** No CORS proxy configured, or proxy URL is incorrect

**Fix:**
1. Make sure you've saved your proxy URL in Settings
2. Reload the page after saving
3. Test your proxy: `https://your-worker.workers.dev/?url=https://www.google.com`

### "Proxy error: Failed to fetch"

**Cause:** The tracker is unreachable or blocking requests

**Fix:**
- Try a different tracker (some trackers have anti-bot protection)
- Check that the tracker URL in your torrent file is correct

### "Missing url parameter"

**Cause:** Proxy URL is configured incorrectly

**Fix:**
- Make sure your proxy URL doesn't end with a `/`
- Correct: `https://my-proxy.workers.dev`
- Wrong: `https://my-proxy.workers.dev/`

### Announcements are slow

**Cause:** Proxy adds a small network hop

**Fix:**
- Normal. Web version is ~50-100ms slower than desktop
- For best performance, use the desktop app

## 🎓 Understanding the Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Your Browser                         │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │         Rustatio Web App (WebAssembly)                │ │
│  │  - Parses torrent files                               │ │
│  │  - Generates peer IDs                                 │ │
│  │  - Tracks fake upload/download stats                  │ │
│  └────────────┬──────────────────────────────────────────┘ │
│               │                                             │
└───────────────┼─────────────────────────────────────────────┘
                │
                │ 1. Announce request
                ↓
    ┌───────────────────────────┐
    │  Your Cloudflare Worker   │
    │     (CORS Proxy)          │
    │  - Adds CORS headers      │
    │  - Forwards requests      │
    └────────────┬──────────────┘
                 │
                 │ 2. Forward to tracker
                 ↓
        ┌────────────────────┐
        │  BitTorrent Tracker │
        │  (e.g., nyaa.si)   │
        └────────────────────┘
```

## 🔒 Privacy & Security

### What data does the proxy see?

Your CORS proxy sees:
- Torrent info hash (identifies the torrent)
- Your fake upload/download statistics
- The tracker URL

**The proxy does NOT see:**
- Actual torrent file contents
- Your IP address (tracker sees it, but proxy doesn't log it)
- Your browsing history or other activities

### Is this legal?

The CORS proxy itself is perfectly legal—it's just a tool for making HTTP requests. However:

- ⚠️ **Faking tracker statistics** may violate the terms of service of private trackers
- ⚠️ Use responsibly and at your own risk
- ⚠️ This tool is for **educational purposes**

## 📚 Additional Resources

- **Desktop App:** No CORS issues, better performance
- **Source Code:** https://github.com/yourusername/rustatio
- **Issues/Support:** https://github.com/yourusername/rustatio/issues
- **Cloudflare Workers Docs:** https://developers.cloudflare.com/workers/

---

**Enjoy using Rustatio! 🎉**

*For the best experience with no limitations, consider using the [desktop app](https://github.com/yourusername/rustatio/releases).*
