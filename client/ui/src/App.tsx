import { useState } from "react";
import type { Engine, increment } from "../../pkg/client.d.ts";
import "./App.css";
import { AudioPlayer } from "./AudioPlayer";

declare global {
  interface Window {
    engine: Engine;
    increment: typeof increment;
  }
}

function App() {
  const [count, setCount] = useState(0);

  const handleClick = () => {
    const nextValue = window.increment(count);
    setCount(nextValue);
  };

  return (
    <>
      <h1>oh hey triangle this is react, what's up??</h1>
      <div className="card">
        <button onClick={handleClick}>count from Rust is {count}</button>
        <AudioPlayer />
      </div>
    </>
  );
}


export default App;
