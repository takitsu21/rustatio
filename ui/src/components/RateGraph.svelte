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
      return;
    }

    // Check for any dark theme (default dark or Catppuccin dark variants)
    const root = document.documentElement;
    const isDark =
      root.classList.contains('dark') ||
      root.classList.contains('frappe') ||
      root.classList.contains('macchiato') ||
      root.classList.contains('mocha');

    // Use CSS custom properties for theme-aware colors
    const computedStyle = getComputedStyle(root);
    const textColor =
      computedStyle.getPropertyValue('--foreground').trim() || (isDark ? '#e5e7eb' : '#1f2937');
    const gridColor =
      computedStyle.getPropertyValue('--border').trim() || (isDark ? '#374151' : '#e5e7eb');
    const mutedBg =
      computedStyle.getPropertyValue('--muted').trim() || (isDark ? '#1f2937' : '#f3f4f6');
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
        backgroundColor: mutedBg,
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
        top: 5,
        itemGap: 20,
      },
      grid: {
        left: '8%',
        right: '8%',
        bottom: '18%',
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
          name: 'Ratio',
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
          symbolSize: 4,
          lineStyle: {
            color: '#22c55e',
            width: 2,
          },
          itemStyle: {
            color: '#22c55e',
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(34, 197, 94, 0.25)' },
              { offset: 1, color: 'rgba(34, 197, 94, 0.02)' },
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
          symbolSize: 4,
          lineStyle: {
            color: '#3b82f6',
            width: 2,
          },
          itemStyle: {
            color: '#3b82f6',
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(59, 130, 246, 0.25)' },
              { offset: 1, color: 'rgba(59, 130, 246, 0.02)' },
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
          symbolSize: 5,
          lineStyle: {
            color: '#f59e0b',
            width: 2,
            type: 'dashed',
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
          backgroundColor: mutedBg,
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
          height: 20,
        },
      ],
    };

    // Use silent mode to update without triggering events/redraws
    chart.setOption(option, false, false);
  }
</script>

<Card class="p-3">
  <div class="flex items-center justify-between mb-3">
    <h2 class="text-primary text-lg font-semibold flex items-center gap-2">
      <Activity size={20} /> Performance
    </h2>
    {#if userHasZoomed}
      <button
        onclick={resetZoom}
        class="flex items-center gap-1.5 px-2 py-1 text-xs bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground rounded border border-border transition-colors cursor-pointer"
        title="Reset zoom to show all data"
      >
        <RotateCcw size={12} />
        Reset Zoom
      </button>
    {/if}
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
      <div
        bind:this={chartContainer}
        class="w-full h-[220px] bg-muted/30 rounded-lg border border-border"
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
