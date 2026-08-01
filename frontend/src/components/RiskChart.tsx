import {
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
} from "recharts";

import { COMMUNITIES, riskScore } from "../lib/riskModel";

export default function RiskChart() {
  const data = COMMUNITIES.map((c) => ({
    name: c.name,
    risk: Number(riskScore(c).toFixed(3)),
  }));

  return (
    <div className="card" style={{ padding: 24 }}>
      <h2 style={{ marginBottom: 20 }}>Community Risk Comparison</h2>

      <div style={{ width: "100%", height: 320 }}>
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="name" />
            <YAxis />
            <Tooltip />
            <Bar
              dataKey="risk"
              fill="#ef4444"
              radius={[6, 6, 0, 0]}
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
