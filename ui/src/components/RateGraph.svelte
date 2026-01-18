<script>
  import { onMount, onDestroy } from 'svelte';
  import * as echarts from 'echarts';
  import Card from '$lib/components/ui/card.svelte';
  import { Activity, Users, Upload, Download, Percent, Clock, RotateCcw } from '@lucide/svelte';

  let { stats, formatDuration } = $props();

  let chartContainer = $state();
  let chart = $state();
  let currentZoom = $state({ start: 0, end: 100 });
  let userHasZoomed = $state(false);
  let lastDataLength = $state(0);

  // Track timestamps for each data point
  let timestamps = $state([]);

  // Colors
  const COLORS = {
    upload: '#10b981', // emerald-500
    uploadLight: 'rgba(16, 185, 129, 0.15)',
    download: '#3b82f6', // blue-500
    downloadLight: 'rgba(59, 130, 246, 0.15)',
    ratio: '#f59e0b', // amber-500
    primary: '#8b5cf6', // violet-500
  };

  // Format time for tooltip
  function formatTimeTooltip(date) {
    return date.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  }

  // Calculate statistics for a data array
  function calcStats(data) {
    if (!data || data.length === 0) return { min: 0, max: 0, avg: 0 };
    const min = Math.min(...data);
    const max = Math.max(...data);
    const avg = data.reduce((a, b) => a + b, 0) / data.length;
    return { min, max, avg };
  }

  // Derived statistics
  const uploadStats = $derived(calcStats(stats?.upload_rate_history));
  const downloadStats = $derived(calcStats(stats?.download_rate_history));

  // Update timestamps when new data points are added
  $effect(() => {
    if (stats?.upload_rate_history) {
      const currentLength = stats.upload_rate_history.length;
      const timestampLength = timestamps.length;

      if (currentLength > timestampLength) {
        // Add timestamps for new data points
        const newTimestamps = [...timestamps];
        for (let i = timestampLength; i < currentLength; i++) {
          newTimestamps.push(new Date());
        }
        timestamps = newTimestamps;
      } else if (currentLength < timestampLength) {
        // Data was reset, clear timestamps
        timestamps = [];
        for (let i = 0; i < currentLength; i++) {
          timestamps.push(new Date());
        }
      }
    }
  });

  onMount(() => {
    setTimeout(() => {
      if (chartContainer) {
        chart = echarts.init(chartContainer);

        chart.on('dataZoom', params => {
          const option = chart.getOption();
          if (option.dataZoom && option.dataZoom[0]) {
            currentZoom = {
              start: option.dataZoom[0].start,
              end: option.dataZoom[0].end,
            };
            if (params.batch && params.batch.length > 0) {
              userHasZoomed = true;
            }
          }
        });

        updateChart();
        window.addEventListener('resize', handleResize);
      }
    }, 100);
  });

  onDestroy(() => {
    if (chart) {
      chart.dispose();
    }
    window.removeEventListener('resize', handleResize);
  });

  function handleResize() {
    if (chart) {
      chart.resize();
    }
  }

  function resetZoom() {
    userHasZoomed = false;
    currentZoom = { start: 0, end: 100 };
    if (chart) {
      updateChart();
    }
  }

  $effect(() => {
    if (chart && stats) {
      updateChart();
    }
  });

  function updateChart() {
    if (!chart || !stats || !stats.upload_rate_history || stats.upload_rate_history.length === 0) {
      return;
    }

    const root = document.documentElement;
    const isDark =
      root.classList.contains('dark') ||
      root.classList.contains('frappe') ||
      root.classList.contains('macchiato') ||
      root.classList.contains('mocha');

    const textColor = isDark ? 'rgba(255, 255, 255, 0.7)' : 'rgba(0, 0, 0, 0.6)';
    const textColorStrong = isDark ? 'rgba(255, 255, 255, 0.9)' : 'rgba(0, 0, 0, 0.85)';
    const gridColor = isDark ? 'rgba(255, 255, 255, 0.06)' : 'rgba(0, 0, 0, 0.06)';
    const axisLineColor = isDark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)';
    const tooltipBg = isDark ? 'rgba(24, 24, 27, 0.95)' : 'rgba(255, 255, 255, 0.95)';
    const tooltipBorder = isDark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)';

    const dataLength = stats.upload_rate_history.length;

    if (!userHasZoomed && lastDataLength === 0 && dataLength > 0) {
      currentZoom = { start: 0, end: 100 };
    }
    lastDataLength = dataLength;

    // Capture timestamps in closure for tooltip formatter
    const capturedTimestamps = [...timestamps];

    // Create time-series data: [timestamp, value] pairs
    const uploadData = stats.upload_rate_history.map((value, i) => {
      const time = capturedTimestamps[i] ? capturedTimestamps[i].getTime() : Date.now();
      return [time, value];
    });
    const downloadData = stats.download_rate_history.map((value, i) => {
      const time = capturedTimestamps[i] ? capturedTimestamps[i].getTime() : Date.now();
      return [time, value];
    });
    const ratioData = (stats.ratio_history || []).map((value, i) => {
      const time = capturedTimestamps[i] ? capturedTimestamps[i].getTime() : Date.now();
      return [time, value];
    });

    const option = {
      backgroundColor: 'transparent',
      animation: false,
      tooltip: {
        trigger: 'axis',
        backgroundColor: tooltipBg,
        borderColor: tooltipBorder,
        borderWidth: 1,
        borderRadius: 8,
        padding: [12, 16],
        textStyle: {
          color: textColorStrong,
          fontSize: 12,
        },
        axisPointer: {
          type: 'line',
          lineStyle: {
            color: COLORS.primary,
            width: 1,
            type: 'dashed',
          },
          crossStyle: {
            color: COLORS.primary,
          },
        },
        formatter: function (params) {
          const timestamp = new Date(params[0].value[0]);
          const timeLabel = formatTimeTooltip(timestamp);
          let result = `<div style="font-size: 11px; color: ${textColor}; margin-bottom: 8px; font-weight: 500;">${timeLabel}</div>`;
          result += '<div style="display: flex; flex-direction: column; gap: 6px;">';
          params.forEach(param => {
            const value = param.value[1].toFixed(2);
            const unit = param.seriesName === 'Ratio' ? '' : ' KB/s';
            const color = param.color;
            result += `<div style="display: flex; align-items: center; justify-content: space-between; gap: 16px;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="width: 8px; height: 8px; border-radius: 2px; background: ${color};"></span>
                <span style="font-size: 12px; color: ${textColor};">${param.seriesName}</span>
              </div>
              <span style="font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums;">${value}${unit}</span>
            </div>`;
          });
          result += '</div>';
          return result;
        },
      },
      legend: {
        show: false,
      },
      grid: {
        left: 12,
        right: 12,
        bottom: 48,
        top: 12,
        containLabel: true,
      },
      xAxis: [
        {
          type: 'time',
          boundaryGap: false,
          axisLine: {
            show: true,
            lineStyle: {
              color: axisLineColor,
            },
          },
          axisTick: {
            show: false,
          },
          axisLabel: {
            color: textColor,
            fontSize: 10,
            margin: 8,
            formatter: {
              hour: '{HH}:{mm}',
              minute: '{HH}:{mm}',
              second: '{HH}:{mm}:{ss}',
            },
            hideOverlap: true,
          },
          splitLine: {
            show: true,
            lineStyle: {
              color: gridColor,
              type: 'dashed',
            },
          },
        },
      ],
      yAxis: [
        {
          type: 'value',
          position: 'left',
          axisLine: {
            show: false,
          },
          axisTick: {
            show: false,
          },
          axisLabel: {
            color: textColor,
            fontSize: 10,
            margin: 8,
            formatter: value => {
              if (value >= 1000) return (value / 1000).toFixed(1) + 'k';
              return value.toFixed(0);
            },
          },
          splitLine: {
            lineStyle: {
              color: gridColor,
              type: 'dashed',
            },
          },
          splitNumber: 4,
        },
        {
          type: 'value',
          position: 'right',
          axisLine: {
            show: false,
          },
          axisTick: {
            show: false,
          },
          axisLabel: {
            color: textColor,
            fontSize: 10,
            margin: 8,
            formatter: value => value.toFixed(1),
          },
          splitLine: {
            show: false,
          },
          splitNumber: 4,
        },
      ],
      series: [
        {
          name: 'Upload',
          type: 'line',
          smooth: 0.3,
          symbol: 'none',
          sampling: 'lttb',
          lineStyle: {
            color: COLORS.upload,
            width: 2,
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: COLORS.uploadLight },
              { offset: 1, color: 'rgba(16, 185, 129, 0)' },
            ]),
          },
          emphasis: {
            focus: 'series',
            lineStyle: {
              width: 2.5,
            },
          },
          data: uploadData,
          yAxisIndex: 0,
        },
        {
          name: 'Download',
          type: 'line',
          smooth: 0.3,
          symbol: 'none',
          sampling: 'lttb',
          lineStyle: {
            color: COLORS.download,
            width: 2,
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: COLORS.downloadLight },
              { offset: 1, color: 'rgba(59, 130, 246, 0)' },
            ]),
          },
          emphasis: {
            focus: 'series',
            lineStyle: {
              width: 2.5,
            },
          },
          data: downloadData,
          yAxisIndex: 0,
        },
        {
          name: 'Ratio',
          type: 'line',
          smooth: 0.3,
          symbol: 'none',
          sampling: 'lttb',
          lineStyle: {
            color: COLORS.ratio,
            width: 1.5,
            type: [4, 4],
          },
          emphasis: {
            focus: 'series',
            lineStyle: {
              width: 2,
            },
          },
          data: ratioData,
          yAxisIndex: 1,
        },
      ],
      dataZoom: [
        {
          type: 'inside',
          start: currentZoom.start,
          end: currentZoom.end,
          filterMode: 'none',
          zoomLock: false,
          moveOnMouseMove: true,
          moveOnMouseWheel: true,
          preventDefaultMouseMove: true,
        },
        {
          type: 'slider',
          show: true,
          start: currentZoom.start,
          end: currentZoom.end,
          height: 20,
          bottom: 8,
          borderColor: 'transparent',
          backgroundColor: isDark ? 'rgba(255, 255, 255, 0.03)' : 'rgba(0, 0, 0, 0.03)',
          fillerColor: isDark ? 'rgba(139, 92, 246, 0.2)' : 'rgba(139, 92, 246, 0.15)',
          handleIcon:
            'path://M-9.35,34.56V42m0-40V9.5m-2,0h4a2,2,0,0,1,2,2v21a2,2,0,0,1-2,2h-4a2,2,0,0,1-2-2v-21A2,2,0,0,1-11.35,9.5Z',
          handleSize: '80%',
          handleStyle: {
            color: COLORS.primary,
            borderColor: COLORS.primary,
          },
          moveHandleStyle: {
            color: COLORS.primary,
          },
          textStyle: {
            color: textColor,
            fontSize: 10,
          },
          brushSelect: false,
          emphasis: {
            handleStyle: {
              color: COLORS.primary,
              borderColor: COLORS.primary,
            },
          },
          dataBackground: {
            lineStyle: {
              color: isDark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)',
            },
            areaStyle: {
              color: isDark ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 0, 0, 0.05)',
            },
          },
          selectedDataBackground: {
            lineStyle: {
              color: COLORS.primary,
            },
            areaStyle: {
              color: isDark ? 'rgba(139, 92, 246, 0.2)' : 'rgba(139, 92, 246, 0.15)',
            },
          },
        },
      ],
    };

    chart.setOption(option, false, false);
  }
</script>

<Card class="p-3">
  <div class="flex items-center justify-between mb-3">
    <h2 class="text-primary text-lg font-semibold flex items-center gap-2">
      <Activity size={20} /> Performance
    </h2>
    <div class="flex items-center gap-2">
      {#if userHasZoomed}
        <button
          onclick={resetZoom}
          class="flex items-center gap-1.5 px-2 py-1 text-xs bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground rounded border border-border transition-colors cursor-pointer"
          title="Reset zoom to show all data"
        >
          <RotateCcw size={12} />
          Reset
        </button>
      {/if}
    </div>
  </div>

  <!-- Live Stats Bar -->
  {#if stats}
    <div class="bg-muted/50 rounded-lg border border-border overflow-hidden mb-3">
      <div class="grid grid-cols-4">
        <div class="p-3 border-r border-border">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
            <Upload size={12} class="text-stat-upload" />
            Upload
          </div>
          <div class="text-lg font-bold text-stat-upload">
            {stats.current_upload_rate.toFixed(1)}
            <span class="text-xs font-normal text-muted-foreground">KB/s</span>
          </div>
        </div>
        <div class="p-3 border-r border-border">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
            <Download size={12} class="text-stat-download" />
            Download
          </div>
          <div class="text-lg font-bold text-stat-download">
            {stats.current_download_rate.toFixed(1)}
            <span class="text-xs font-normal text-muted-foreground">KB/s</span>
          </div>
        </div>
        <div class="p-3 border-r border-border">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
            <Percent size={12} class="text-stat-ratio" />
            Ratio
          </div>
          <div class="text-lg font-bold text-stat-ratio">
            {stats.ratio.toFixed(2)}
          </div>
        </div>
        <div class="p-3">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
            <Clock size={12} />
            Elapsed
          </div>
          <div class="text-lg font-bold">
            {formatDuration(stats.elapsed_time?.secs || 0)}
          </div>
        </div>
      </div>
    </div>
  {/if}

  <div class="grid grid-cols-1 lg:grid-cols-4 gap-3">
    <!-- Performance Chart -->
    <div class="lg:col-span-3">
      <!-- Custom Legend -->
      {#if stats && stats.upload_rate_history && stats.upload_rate_history.length > 0}
        <div class="flex items-center gap-4 mb-2 px-1">
          <div class="flex items-center gap-1.5">
            <span class="w-3 h-0.5 rounded-full bg-emerald-500"></span>
            <span class="text-xs text-muted-foreground">Upload</span>
            <span class="text-xs font-medium text-emerald-500 tabular-nums">
              avg {uploadStats.avg.toFixed(1)} KB/s
            </span>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="w-3 h-0.5 rounded-full bg-blue-500"></span>
            <span class="text-xs text-muted-foreground">Download</span>
            <span class="text-xs font-medium text-blue-500 tabular-nums">
              avg {downloadStats.avg.toFixed(1)} KB/s
            </span>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="w-3 h-0.5 rounded-full bg-amber-500" style="border-style: dashed;"></span>
            <span class="text-xs text-muted-foreground">Ratio</span>
          </div>
        </div>
      {/if}

      <div
        bind:this={chartContainer}
        class="w-full h-[220px] bg-muted/20 rounded-lg border border-border"
      >
        {#if !stats || !stats.upload_rate_history || stats.upload_rate_history.length === 0}
          <div class="w-full h-full flex items-center justify-center">
            <div class="text-center">
              <Activity size={32} class="text-muted-foreground mx-auto mb-2 opacity-50" />
              <p class="text-sm text-muted-foreground">Waiting for data...</p>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <!-- Peer Distribution -->
    <div class="lg:col-span-1">
      {#if stats}
        {@const total = stats.seeders + stats.leechers}
        {@const seederPercent = total > 0 ? (stats.seeders / total) * 100 : 50}
        {@const leecherPercent = total > 0 ? (stats.leechers / total) * 100 : 50}

        <div class="bg-muted/50 rounded-lg border border-border p-3 h-[220px] flex flex-col">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-3">
            <Users size={12} />
            Peers
          </div>

          <!-- Total count -->
          <div class="text-center mb-3">
            <div class="text-3xl font-bold">{total}</div>
            <div class="text-xs text-muted-foreground">connected</div>
          </div>

          <!-- Peer bars -->
          <div class="flex-1 flex flex-col justify-center gap-3">
            <!-- Seeders -->
            <div>
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs text-muted-foreground">Seeders</span>
                <span class="text-xs font-bold text-stat-upload">{stats.seeders}</span>
              </div>
              <div class="w-full h-2 bg-background rounded-full overflow-hidden">
                <div
                  class="h-full bg-stat-upload rounded-full transition-all duration-300"
                  style="width: {seederPercent}%"
                ></div>
              </div>
            </div>

            <!-- Leechers -->
            <div>
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs text-muted-foreground">Leechers</span>
                <span class="text-xs font-bold text-stat-danger">{stats.leechers}</span>
              </div>
              <div class="w-full h-2 bg-background rounded-full overflow-hidden">
                <div
                  class="h-full bg-stat-danger rounded-full transition-all duration-300"
                  style="width: {leecherPercent}%"
                ></div>
              </div>
            </div>
          </div>

          <!-- Ratio indicator -->
          <div class="pt-2 mt-2 border-t border-border text-center">
            <span class="text-xs text-muted-foreground">S/L Ratio: </span>
            <span
              class="text-xs font-bold {stats.leechers > 0
                ? 'text-primary'
                : 'text-muted-foreground'}"
            >
              {stats.leechers > 0 ? (stats.seeders / stats.leechers).toFixed(1) : '∞'}
            </span>
          </div>
        </div>
      {:else}
        <div
          class="bg-muted/50 rounded-lg border border-border p-3 h-[220px] flex items-center justify-center"
        >
          <div class="text-center">
            <Users size={24} class="text-muted-foreground mx-auto mb-2 opacity-50" />
            <p class="text-xs text-muted-foreground">No peer data</p>
          </div>
        </div>
      {/if}
    </div>
  </div>
</Card>
