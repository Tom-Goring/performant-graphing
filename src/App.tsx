import "./App.css";
import { useEffect, useRef, useState } from "react";
import TimeChart from "timechart";

function App() {
  const chartRef = useRef<any | null>(null);
  const chart = useRef<TimeChart | null>(null);
  const [ws, setWs] = useState(null);
  const [name, setName] = useState("");

  useEffect(() => {
    chart.current = new TimeChart(chartRef.current!, {
      series: [],
      zoom: {
        x: { autoRange: false, maxDomainExtent: 100 },
        y: { autoRange: false },
      },
    });

    const ws = new WebSocket("ws://localhost:4000/ws");

    ws.onmessage = (e) => {
      const incoming = JSON.parse(e.data);

      for (const point in incoming) {
        let s = chart.current?.options.series.find((s) => s.name == point);
        if (!s) {
          let color = `#${Math.floor(Math.random() * 16777215).toString(16)}`;
          console.log(
            `Found no series with label ${point}, creating a new one with first point at ${incoming[point]} and color ${color}`
          );
          chart.current?.options.series.push({
            name: point,
            data: [{ x: 1, y: incoming[point] }],
            color,
            visible: true,
            _complete: true,
          });
          chart.current?.update();
        } else {
          s.data.push({ x: s.data.length + 1, y: incoming[point] });
        }
      }

      chart.current!.update();
    };

    return () => {
      chart.current!.dispose();
    };
  }, []);

  const followData = () => {
    chart.current!.options.realTime = true;
  };

  const addSeries = () => {
    fetch("/series", {
      method: "POST",
      body: JSON.stringify({ name: name }),
      headers: { "content-type": "application/json" },
    });
  };

  return (
    <div>
      <div ref={chartRef} style={{ width: "100%", height: 500 }} />
      <button onClick={followData}>Follow</button>
      <hr />
      <input
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="New Series Name"
      />
      <button onClick={addSeries}>Add Series</button>
    </div>
  );
}

export default App;
