<script>
  import { onMount, onDestroy } from 'svelte';
  import * as echarts from 'echarts';
  import Card from '$lib/components/ui/card.svelte';

  let { stats, formatDuration } = $props();

  let chartContainer = $state();
  let chart = $state();
  let currentZoom = $state({ start: 0, end: 100 });
  let userHasZoomed = $state(false);
  let lastDataLength = $state(0);

  onMount(() => {
    setTimeout(() => {
      if (chartContainer) {
        console.log('Initializing ECharts...', { chartContainer, stats });
        chart = echarts.init(chartContainer);

        chart.on('dataZoom', params => {
          const option = chart.getOption();
          if (option.dataZoom && option.dataZoom[0]) {
            currentZoom = {
              start: option.dataZoom[0].start,
              end: option.dataZoom[0].end,
            };
            // Mark that user has manually zoomed if this wasn't triggered by our auto-scroll
            if (params.batch && params.batch.length > 0) {
              userHasZoomed = true;
            }
          }
        });

        updateChart();

        window.addEventListener('resize', handleResize);
      } else {
        console.error('Chart container not found');
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
      console.log('Cannot update chart:', {
        hasChart: !!chart,
        hasStats: !!stats,
        hasHistory: !!stats?.upload_rate_history,
        historyLength: stats?.upload_rate_history?.length,
      });
      return;
    }

    console.log('Updating chart with data points:', stats.upload_rate_history.length);

    const isDark = document.documentElement.classList.contains('dark');
    const textColor = isDark ? '#e5e7eb' : '#1f2937';
    const gridColor = isDark ? '#374151' : '#e5e7eb';
    const backgroundColor = 'transparent';

    const xAxisData = stats.upload_rate_history.map((_, i) => i + 1);

    // Track data length changes but don't modify zoom on every update
    const dataLength = stats.upload_rate_history.length;

    // Only reset zoom when starting fresh (data went from 0 to some value)
    if (!userHasZoomed && lastDataLength === 0 && dataLength > 0) {
      currentZoom = { start: 0, end: 100 };
    }
    lastDataLength = dataLength;

    const option = {
      backgroundColor: backgroundColor,
      animation: false, // Disable animations to prevent chart redrawing
      tooltip: {
        trigger: 'axis',
        backgroundColor: isDark ? '#1f2937' : '#ffffff',
        borderColor: '#7c3aed',
        borderWidth: 2,
        textStyle: {
          color: textColor,
        },
        axisPointer: {
          type: 'cross',
          label: {
            backgroundColor: '#7c3aed',
          },
        },
        formatter: function (params) {
          let result = `<div style="font-weight: bold; margin-bottom: 4px;">Point ${params[0].axisValue}</div>`;
          params.forEach(param => {
            const value = param.value.toFixed(2);
            const unit = param.seriesName === 'Ratio' ? '' : ' KB/s';
            result += `<div style="display: flex; align-items: center; gap: 8px;">
              <span style="display: inline-block; width: 10px; height: 10px; border-radius: 50%; background-color: ${param.color};"></span>
              <span>${param.seriesName}: ${value}${unit}</span>
            </div>`;
          });
          return result;
        },
      },
      legend: {
        data: ['Upload', 'Download', 'Ratio'],
        textStyle: {
          color: textColor,
        },
        top: 10,
      },
      grid: {
        left: '3%',
        right: '4%',
        bottom: '15%',
        top: '15%',
        containLabel: true,
      },
      xAxis: [
        {
          type: 'category',
          boundaryGap: false,
          data: xAxisData,
          axisLine: {
            lineStyle: {
              color: gridColor,
            },
          },
          axisLabel: {
            color: textColor,
            interval: Math.floor(xAxisData.length / 10) || 1,
          },
          splitLine: {
            show: true,
            lineStyle: {
              color: gridColor,
              opacity: 0.2,
            },
          },
        },
      ],
      yAxis: [
        {
          type: 'value',
          name: 'Rate (KB/s)',
          position: 'left',
          nameTextStyle: {
            color: textColor,
          },
          axisLine: {
            lineStyle: {
              color: gridColor,
            },
          },
          axisLabel: {
            color: textColor,
            formatter: '{value}',
          },
          splitLine: {
            lineStyle: {
              color: gridColor,
              opacity: 0.2,
            },
          },
        },
        {
          type: 'value',
          name: 'Ratio (Upload/Total)',
          position: 'right',
          nameTextStyle: {
            color: textColor,
          },
          axisLine: {
            lineStyle: {
              color: gridColor,
            },
          },
          axisLabel: {
            color: textColor,
            formatter: '{value}',
          },
          splitLine: {
            show: false,
          },
        },
      ],
      series: [
        {
          name: 'Upload',
          type: 'line',
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          lineStyle: {
            color: '#10b981',
            width: 3,
          },
          itemStyle: {
            color: '#10b981',
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(16, 185, 129, 0.3)' },
              { offset: 1, color: 'rgba(16, 185, 129, 0.05)' },
            ]),
          },
          data: stats.upload_rate_history,
          yAxisIndex: 0,
        },
        {
          name: 'Download',
          type: 'line',
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          lineStyle: {
            color: '#6366f1',
            width: 3,
            type: 'dashed',
          },
          itemStyle: {
            color: '#6366f1',
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(99, 102, 241, 0.3)' },
              { offset: 1, color: 'rgba(99, 102, 241, 0.05)' },
            ]),
          },
          data: stats.download_rate_history,
          yAxisIndex: 0,
        },
        {
          name: 'Ratio',
          type: 'line',
          smooth: true,
          symbol: 'diamond',
          symbolSize: 6,
          lineStyle: {
            color: '#f59e0b',
            width: 2,
            type: 'dotted',
          },
          itemStyle: {
            color: '#f59e0b',
          },
          data: stats.ratio_history || [],
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
          start: currentZoom.start,
          end: currentZoom.end,
          backgroundColor: isDark ? '#1f2937' : '#f3f4f6',
          fillerColor: 'rgba(124, 58, 237, 0.3)',
          borderColor: gridColor,
          textStyle: {
            color: textColor,
          },
          handleStyle: {
            color: '#7c3aed',
          },
          moveHandleStyle: {
            color: '#7c3aed',
          },
          brushSelect: false,
          zoomLock: false,
        },
      ],
    };

    // Use silent mode to update without triggering events/redraws
    chart.setOption(option, false, false);
  }
</script>

<Card class="p-3">
  <div class="flex justify-between items-center mb-3">
    <h2 class="text-primary text-lg font-semibold">📊 Performance & Peer Analytics</h2>
    {#if userHasZoomed}
      <button
        onclick={resetZoom}
        class="px-3 py-1 text-sm bg-primary text-white rounded hover:bg-primary/90 transition-colors"
        title="Reset zoom to show all data"
      >
        Reset Zoom
      </button>
    {/if}
  </div>

  <!-- Live Stats Grid -->
  {#if stats}
    <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-3">
      <div class="bg-muted p-3 rounded-lg text-center border border-border">
        <span
          class="block text-xs text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
          >↑ Upload</span
        >
        <span class="block text-base font-bold text-green-500"
          >{stats.current_upload_rate.toFixed(1)} KB/s</span
        >
      </div>
      <div class="bg-muted p-3 rounded-lg text-center border border-border">
        <span
          class="block text-xs text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
          >↓ Download</span
        >
        <span class="block text-base font-bold text-indigo-500"
          >{stats.current_download_rate.toFixed(1)} KB/s</span
        >
      </div>
      <div class="bg-muted p-3 rounded-lg text-center border border-border">
        <span
          class="block text-xs text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
          >Ratio</span
        >
        <span class="block text-base font-bold text-amber-500">{stats.ratio.toFixed(2)}</span>
      </div>
      <div class="bg-muted p-3 rounded-lg text-center border border-border">
        <span
          class="block text-xs text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
          >Elapsed Time</span
        >
        <span class="block text-base font-bold text-foreground"
          >{formatDuration(stats.elapsed_time?.secs || 0)}</span
        >
      </div>
    </div>
  {/if}

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
    <!-- Performance Chart (2/3 width) -->
    <div class="lg:col-span-2">
      <div
        bind:this={chartContainer}
        class="w-full h-[250px] bg-muted rounded-lg border border-border p-3"
      >
        {#if !stats || !stats.upload_rate_history || stats.upload_rate_history.length === 0}
          <div class="w-full h-full flex items-center justify-center">
            <p class="text-foreground text-lg">Waiting for data...</p>
          </div>
        {/if}
      </div>
    </div>

    <!-- Peer Distribution (1/3 width) -->
    <div class="flex flex-col gap-4">
      {#if stats}
        {@const total = stats.seeders + stats.leechers}
        {@const seederPercent = total > 0 ? (stats.seeders / total) * 100 : 50}
        {@const leecherPercent = total > 0 ? (stats.leechers / total) * 100 : 50}

        <div class="bg-muted rounded-lg border border-border p-3 h-[250px] flex flex-col">
          <h3 class="text-sm font-semibold text-primary mb-2">🌐 Peer Distribution</h3>
          <div class="flex flex-col justify-center gap-3 flex-1">
            <!-- Total Peers -->
            <div class="text-center">
              <div class="text-2xl font-bold text-foreground">{total}</div>
              <div class="text-xs text-muted-foreground">Total Peers</div>
            </div>

            <!-- Horizontal Bar Chart -->
            <div class="flex flex-col gap-3">
              <!-- Seeders Bar -->
              <div>
                <div class="flex items-center justify-between mb-1">
                  <span class="text-xs text-muted-foreground font-semibold">Seeders</span>
                  <span class="text-xs text-green-500 font-bold">{stats.seeders}</span>
                </div>
                <div
                  class="w-full h-6 bg-background rounded-full overflow-hidden border border-border"
                >
                  <div
                    class="h-full bg-gradient-to-r from-green-600 to-green-500 flex items-center justify-end pr-2 transition-all duration-300"
                    style="width: {seederPercent}%"
                  >
                    <span class="text-xs text-white font-semibold">{seederPercent.toFixed(0)}%</span
                    >
                  </div>
                </div>
              </div>

              <!-- Leechers Bar -->
              <div>
                <div class="flex items-center justify-between mb-1">
                  <span class="text-xs text-muted-foreground font-semibold">Leechers</span>
                  <span class="text-xs text-red-500 font-bold">{stats.leechers}</span>
                </div>
                <div
                  class="w-full h-6 bg-background rounded-full overflow-hidden border border-border"
                >
                  <div
                    class="h-full bg-gradient-to-r from-red-600 to-red-500 flex items-center justify-end pr-2 transition-all duration-300"
                    style="width: {leecherPercent}%"
                  >
                    <span class="text-xs text-white font-semibold"
                      >{leecherPercent.toFixed(0)}%</span
                    >
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
</Card>
