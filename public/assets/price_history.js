// Price history charts (ECharts). Called from the Rust `PriceHistory` component.
(function () {
  const charts = new Map();
  const observers = new Map();

  function removeChart(elementId) {
    observers.get(elementId)?.disconnect();
    observers.delete(elementId);
    charts.get(elementId)?.dispose();
    charts.delete(elementId);
  }

  function currencyFormat(currency, value) {
    if (currency === "$") {
      return `$${value.toFixed(2)}`;
    }
    return `${currency}${Math.round(value).toLocaleString("ja-JP")}`;
  }

  window.renderPriceHistory = function (elementId, series) {
    if (typeof echarts === "undefined") {
      console.error("ECharts is not loaded");
      return;
    }
    const el = document.getElementById(elementId);
    if (!el) {
      return;
    }

    // the element may have been re-created, so the old chart must be replaced
    let chart = charts.get(elementId);
    if (chart && chart.getDom() !== el) {
      removeChart(elementId);
      chart = null;
    }
    if (!chart) {
      chart = echarts.init(el);
      charts.set(elementId, chart);
      // the container can be sized after init, so the chart follows its size
      const observer = new ResizeObserver(() => chart.resize());
      observer.observe(el);
      observers.set(elementId, observer);
    }

    const currencies = {};
    series.forEach((s) => {
      currencies[s.name] = s.currency;
    });
    const currency = series.length ? series[0].currency : "";

    chart.setOption(
      {
        animation: false,
        grid: { left: 60, right: 20, top: 30, bottom: 30 },
        legend: { show: series.length > 1, top: 0 },
        tooltip: {
          trigger: "axis",
          formatter: (params) => {
            const date = new Date(params[0].value[0]).toLocaleDateString();
            const lines = params.map(
              (p) =>
                `${p.marker} ${p.seriesName}: ${p.value[2] ? ">" : ""}${currencyFormat(currencies[p.seriesName], p.value[1])}`
            );
            return [date, ...lines].join("<br/>");
          },
        },
        xAxis: { type: "time" },
        yAxis: {
          type: "value",
          scale: true,
          axisLabel: {
            formatter: (value) => currencyFormat(currency, value),
          },
        },
        series: series.map((s) => ({
          name: s.name,
          type: "line",
          smooth: 0.3,
          showSymbol: false,
          data: s.data,
        })),
      },
      true
    );
  };

  // reports the first pointer contact (hover or touch) with the chart
  window.watchPriceHistoryUse = function (elementId, channel) {
    const el = document.getElementById(elementId);
    if (!el) {
      return;
    }
    el.addEventListener("pointerenter", () => channel.send(true), { once: true });
  };

  window.disposePriceHistory = function (elementId) {
    removeChart(elementId);
  };
})();
