import assert from 'node:assert/strict';
import test from 'node:test';

import {
  cn,
  detectOS,
  getActiveInstanceIndex,
  getBackendInstanceStateFlags,
  getDownloadType,
  selectActiveInstanceId,
  serializeSessionInstances,
  shouldRetryDesktopRestore,
} from './utils.js';

function withWindow(windowValue, fn) {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'window');

  if (windowValue === undefined) {
    delete globalThis.window;
  } else {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      writable: true,
      value: windowValue,
    });
  }

  try {
    fn();
  } finally {
    if (descriptor) {
      Object.defineProperty(globalThis, 'window', descriptor);
    } else {
      delete globalThis.window;
    }
  }
}

test('cn merges class names and resolves Tailwind conflicts', () => {
  assert.equal(cn('px-2', null, 'px-4', 'py-1'), 'px-4 py-1');
});

test('detectOS returns unknown when window is unavailable', () => {
  withWindow(undefined, () => {
    assert.equal(detectOS(), 'unknown');
  });
});

test('detectOS detects windows from user agent', () => {
  withWindow(
    {
      navigator: {
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
        platform: 'Win32',
      },
    },
    () => {
      assert.equal(detectOS(), 'windows');
    }
  );
});

test('detectOS detects macos from platform', () => {
  withWindow(
    {
      navigator: {
        userAgent: 'Mozilla/5.0',
        platform: 'MacIntel',
      },
    },
    () => {
      assert.equal(detectOS(), 'macos');
    }
  );
});

test('detectOS detects linux from user agent', () => {
  withWindow(
    {
      navigator: {
        userAgent: 'Mozilla/5.0 (X11; Linux x86_64)',
        platform: 'Linux x86_64',
      },
    },
    () => {
      assert.equal(detectOS(), 'linux');
    }
  );
});

test('getDownloadType returns desktop installers for windows and macos', () => {
  assert.equal(getDownloadType('windows'), 'msi');
  assert.equal(getDownloadType('macos'), 'dmg');
});

test('getDownloadType returns deb for ubuntu-like linux environments', () => {
  withWindow(
    {
      navigator: {
        userAgent: 'Mozilla/5.0 Ubuntu Linux',
        platform: 'Linux x86_64',
      },
    },
    () => {
      assert.equal(getDownloadType('linux'), 'deb');
    }
  );
});

test('getDownloadType returns rpm for fedora-like linux environments', () => {
  withWindow(
    {
      navigator: {
        userAgent: 'Mozilla/5.0 Fedora Linux',
        platform: 'Linux x86_64',
      },
    },
    () => {
      assert.equal(getDownloadType('linux'), 'rpm');
    }
  );
});

test('getDownloadType falls back to deb for generic linux and AppImage for unknown os', () => {
  withWindow(
    {
      navigator: {
        userAgent: 'Mozilla/5.0 (X11; Linux x86_64)',
        platform: 'Linux x86_64',
      },
    },
    () => {
      assert.equal(getDownloadType('linux'), 'deb');
    }
  );

  assert.equal(getDownloadType('plan9'), 'AppImage');
});

test('getBackendInstanceStateFlags treats paused and idle as active backend states', () => {
  assert.deepEqual(getBackendInstanceStateFlags('Paused'), {
    isRunning: true,
    isPaused: true,
    isIdling: false,
  });

  assert.deepEqual(getBackendInstanceStateFlags('Idle'), {
    isRunning: true,
    isPaused: false,
    isIdling: true,
  });
});

test('getBackendInstanceStateFlags treats stopped and unknown states as inactive', () => {
  assert.deepEqual(getBackendInstanceStateFlags('Stopped'), {
    isRunning: false,
    isPaused: false,
    isIdling: false,
  });

  assert.deepEqual(getBackendInstanceStateFlags(undefined), {
    isRunning: false,
    isPaused: false,
    isIdling: false,
  });
});

test('serializeSessionInstances preserves backup snapshot shape for desktop/web sessions', () => {
  const snapshot = serializeSessionInstances([
    {
      id: '5',
      torrentPath: '/tmp/example.torrent',
      torrent: { name: 'example' },
      selectedClient: 'qbittorrent',
      selectedClientVersion: '4.6.0',
      uploadRate: '123.5',
      downloadRate: '45.5',
      port: '51413',
      vpnPortSync: true,
      completionPercent: '87.5',
      initialUploaded: '100',
      initialDownloaded: '25',
      cumulativeUploaded: '300',
      cumulativeDownloaded: '50',
      randomizeRates: true,
      randomRangePercent: '10',
      updateIntervalSeconds: '7',
      scrapeInterval: '90',
      stopAtRatioEnabled: true,
      stopAtRatio: '2.5',
      randomizeRatio: true,
      randomRatioRangePercent: '15',
      effectiveStopAtRatio: 2.7,
      stopAtUploadedEnabled: true,
      stopAtUploadedGB: '3.5',
      stopAtDownloadedEnabled: true,
      stopAtDownloadedGB: '1.5',
      stopAtSeedTimeEnabled: true,
      stopAtSeedTimeHours: '12',
      idleWhenNoLeechers: true,
      idleWhenNoSeeders: false,
      postStopAction: 'pause',
      progressiveRatesEnabled: true,
      targetUploadRate: '500',
      targetDownloadRate: '800',
      progressiveDurationHours: '6',
    },
  ]);

  assert.deepEqual(snapshot, [
    {
      torrent_path: '/tmp/example.torrent',
      torrent_name: 'example',
      torrent_data: { name: 'example' },
      selected_client: 'qbittorrent',
      selected_client_version: '4.6.0',
      upload_rate: 123.5,
      download_rate: 45.5,
      port: 51413,
      vpn_port_sync: true,
      completion_percent: 87.5,
      initial_uploaded: 104857600,
      initial_downloaded: 26214400,
      cumulative_uploaded: 314572800,
      cumulative_downloaded: 52428800,
      randomize_rates: true,
      random_range_percent: 10,
      update_interval_seconds: 7,
      scrape_interval: 90,
      stop_at_ratio_enabled: true,
      stop_at_ratio: 2.5,
      randomize_ratio: true,
      random_ratio_range_percent: 15,
      effective_stop_at_ratio: 2.7,
      stop_at_uploaded_enabled: true,
      stop_at_uploaded_gb: 3.5,
      stop_at_downloaded_enabled: true,
      stop_at_downloaded_gb: 1.5,
      stop_at_seed_time_enabled: true,
      stop_at_seed_time_hours: 12,
      idle_when_no_leechers: true,
      idle_when_no_seeders: false,
      post_stop_action: 'pause',
      progressive_rates_enabled: true,
      target_upload_rate: 500,
      target_download_rate: 800,
      progressive_duration_hours: 6,
    },
  ]);
});

test('getActiveInstanceIndex returns null when the active id is missing', () => {
  const restored = [{ id: '2' }, { id: '5' }];

  assert.equal(getActiveInstanceIndex(restored, '99'), null);
  assert.equal(getActiveInstanceIndex(restored, '5'), 1);
});

test('shouldRetryDesktopRestore only retries when prior desktop state exists', () => {
  assert.equal(shouldRetryDesktopRestore(null), false);
  assert.equal(shouldRetryDesktopRestore({}), false);
  assert.equal(shouldRetryDesktopRestore({ activeInstanceId: null, instances: [] }), false);
  assert.equal(shouldRetryDesktopRestore({ activeInstanceId: '5', instances: [] }), false);
  assert.equal(shouldRetryDesktopRestore({ activeInstanceId: null, instances: [{}] }), false);
  assert.equal(
    shouldRetryDesktopRestore({
      activeInstanceId: null,
      instances: [{ torrentPath: '/tmp/a.torrent' }],
    }),
    true
  );
  assert.equal(
    shouldRetryDesktopRestore({
      activeInstanceId: null,
      instances: [{ torrentName: 'ubuntu.iso' }],
    }),
    true
  );
});

test('selectActiveInstanceId prefers saved id when it exists', () => {
  const restored = [{ id: '2' }, { id: '5' }];

  assert.equal(selectActiveInstanceId(restored, { activeInstanceId: '5' }), '5');
});

test('selectActiveInstanceId falls back to saved index for desktop compatibility', () => {
  const restored = [{ id: '2' }, { id: '5' }];

  assert.equal(selectActiveInstanceId(restored, { activeInstanceIndex: 1 }), '5');
});

test('selectActiveInstanceId falls back to first instance when saved selection is missing', () => {
  const restored = [{ id: '2' }, { id: '5' }];

  assert.equal(selectActiveInstanceId(restored, { activeInstanceId: '99' }), '2');
});

test('selectActiveInstanceId prefers saved id over saved index', () => {
  const restored = [{ id: '2' }, { id: '5' }];

  assert.equal(
    selectActiveInstanceId(restored, { activeInstanceId: '5', activeInstanceIndex: 0 }),
    '5'
  );
});

test('selectActiveInstanceId matches ids across number and string types', () => {
  const restored = [{ id: 2 }, { id: 5 }];

  assert.equal(selectActiveInstanceId(restored, { activeInstanceId: '5' }), 5);
});

test('selectActiveInstanceId falls back to first instance when saved index is out of range', () => {
  const restored = [{ id: '2' }, { id: '5' }];

  assert.equal(selectActiveInstanceId(restored, { activeInstanceIndex: 9 }), '2');
});

test('selectActiveInstanceId returns null when no instances exist', () => {
  assert.equal(selectActiveInstanceId([], { activeInstanceId: '5' }), null);
});
