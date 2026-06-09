import type {
  GroupedBarChartModel,
  HorizontalBarChartModel,
  LineChartModel
} from "./views/decision/charts";

function percentLabel(value: number, yMax: number) {
  const percent = value * 100;
  if (yMax <= 0.001) {
    return `${percent.toFixed(2)}%`;
  }
  if (yMax <= 0.01) {
    return `${percent.toFixed(1)}%`;
  }
  return `${Math.round(percent)}%`;
}

function scoreLabel(value: number) {
  return value.toFixed(0);
}

function chartValueLabel(
  value: number,
  valueType: LineChartModel["valueType"],
  yMax: number
) {
  return valueType === "percent" ? percentLabel(value, yMax) : scoreLabel(value);
}

export function SimpleLineChart({
  model,
  height = 300
}: {
  model: LineChartModel;
  height?: number;
}) {
  const width = 920;
  const margins = { top: 14, right: 12, bottom: 56, left: 42 };
  const plotWidth = width - margins.left - margins.right;
  const plotHeight = height - margins.top - margins.bottom;
  const pointCount = Math.max(model.categories.length, 1);
  const yMax = model.maxValue <= 0 ? 1 : model.maxValue;
  const labelStep = Math.max(1, Math.ceil(model.categories.length / 6));

  const x = (index: number) =>
    pointCount === 1
      ? margins.left + plotWidth / 2
      : margins.left + (index / (pointCount - 1)) * plotWidth;
  const y = (value: number) => margins.top + plotHeight - (Math.max(0, value) / yMax) * plotHeight;

  const ticks = Array.from({ length: 5 }, (_, index) => (yMax / 4) * index).reverse();

  return (
    <div className="simple-chart">
      <svg className="simple-chart-svg" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none">
        {ticks.map((tick) => (
          <g key={tick}>
            <line
              x1={margins.left}
              x2={width - margins.right}
              y1={y(tick)}
              y2={y(tick)}
              stroke="#edf1f4"
              strokeWidth="1"
            />
            <text x={margins.left - 8} y={y(tick) + 4} textAnchor="end" className="simple-chart-axis">
              {chartValueLabel(tick, model.valueType, yMax)}
            </text>
          </g>
        ))}

        {model.series.map((series) => {
          const points = series.values.map((value, index) => `${x(index)},${y(value)}`).join(" ");
          const areaPath = `${series.values
            .map((value, index) => `${index === 0 ? "M" : "L"} ${x(index)} ${y(value)}`)
            .join(" ")} L ${x(series.values.length - 1)} ${margins.top + plotHeight} L ${x(0)} ${margins.top + plotHeight} Z`;

          return (
            <g key={series.label}>
              {series.fillColor ? <path d={areaPath} fill={series.fillColor} /> : null}
              <polyline
                fill="none"
                points={points}
                stroke={series.color}
                strokeWidth="3"
                strokeLinejoin="round"
                strokeLinecap="round"
              />
            </g>
          );
        })}

        {model.categories.map((label, index) => {
          if (index % labelStep !== 0 && index !== model.categories.length - 1) {
            return null;
          }

          return (
            <text
              key={`${label}-${index}`}
              x={x(index)}
              y={height - 18}
              textAnchor="middle"
              className="simple-chart-axis"
            >
              {label}
            </text>
          );
        })}
      </svg>

      <div className="simple-chart-legend">
        {model.series.map((series) => (
          <div className="simple-chart-legend-item" key={series.label}>
            <span className="simple-chart-swatch" style={{ background: series.color }} />
            <span>{series.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function SimpleHorizontalBarChart({
  model
}: {
  model: HorizontalBarChartModel;
}) {
  return (
    <div className="simple-hbar-chart">
      {model.rows.map((row) => (
        <div className="simple-hbar-row" key={row.label}>
          <div className="simple-hbar-head">
            <span>{row.label}</span>
            <strong>{row.value.toFixed(1)}</strong>
          </div>
          <div className="track">
            <div
              className="fill"
              style={{
                width: `${Math.max(0, Math.min(100, (row.value / model.maxValue) * 100))}%`,
                background: row.color
              }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}

export function SimpleGroupedBarChart({
  model,
  height = 300
}: {
  model: GroupedBarChartModel;
  height?: number;
}) {
  const width = 920;
  const margins = { top: 16, right: 16, bottom: 76, left: 42 };
  const plotWidth = width - margins.left - margins.right;
  const plotHeight = height - margins.top - margins.bottom;
  const groupCount = Math.max(model.categories.length, 1);
  const seriesCount = Math.max(model.series.length, 1);
  const groupWidth = plotWidth / groupCount;
  const barWidth = Math.min(30, (groupWidth * 0.64) / seriesCount);

  const y = (value: number) => margins.top + plotHeight - (Math.max(0, value) / model.maxValue) * plotHeight;

  return (
    <div className="simple-chart">
      <svg className="simple-chart-svg" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none">
        {Array.from({ length: 5 }, (_, index) => (model.maxValue / 4) * index).reverse().map((tick) => (
          <g key={tick}>
            <line
              x1={margins.left}
              x2={width - margins.right}
              y1={y(tick)}
              y2={y(tick)}
              stroke="#edf1f4"
              strokeWidth="1"
            />
            <text x={margins.left - 8} y={y(tick) + 4} textAnchor="end" className="simple-chart-axis">
              {scoreLabel(tick)}
            </text>
          </g>
        ))}

        {model.categories.map((category, categoryIndex) => {
          const groupStart = margins.left + categoryIndex * groupWidth + groupWidth * 0.18;

          return (
            <g key={`${category}-${categoryIndex}`}>
              {model.series.map((series, seriesIndex) => {
                const value = series.values[categoryIndex] ?? 0;
                const barX = groupStart + seriesIndex * barWidth;
                const barY = y(value);
                const barHeight = margins.top + plotHeight - barY;

                return (
                  <rect
                    key={`${series.label}-${category}`}
                    x={barX}
                    y={barY}
                    width={barWidth - 4}
                    height={barHeight}
                    rx="4"
                    fill={series.color}
                  />
                );
              })}
              <text
                x={groupStart + (seriesCount * barWidth) / 2}
                y={height - 26}
                textAnchor="middle"
                className="simple-chart-axis"
              >
                {category.split("\n").map((part, partIndex) => (
                  <tspan key={`${category}-${partIndex}`} x={groupStart + (seriesCount * barWidth) / 2} dy={partIndex === 0 ? 0 : 14}>
                    {part}
                  </tspan>
                ))}
              </text>
            </g>
          );
        })}
      </svg>

      <div className="simple-chart-legend">
        {model.series.map((series) => (
          <div className="simple-chart-legend-item" key={series.label}>
            <span className="simple-chart-swatch" style={{ background: series.color }} />
            <span>{series.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
