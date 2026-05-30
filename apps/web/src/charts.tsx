import ReactEChartsCore from "echarts-for-react/lib/core";
import type { EChartsReactProps } from "echarts-for-react";
import * as echarts from "echarts/core";
import { BarChart, LineChart } from "echarts/charts";
import {
  AxisPointerComponent,
  GridComponent,
  LegendComponent,
  TooltipComponent
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";

echarts.use([
  LineChart,
  BarChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  AxisPointerComponent,
  CanvasRenderer
]);

export default function ReactECharts(props: EChartsReactProps) {
  return <ReactEChartsCore echarts={echarts} {...props} />;
}
