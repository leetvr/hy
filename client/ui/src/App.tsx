import { useState } from "react";
import "./App.css";

function App() {
  const [count, setCount] = useState(0);

  const handleClick = () => {
    const nextValue = (window as unknown).increment(count);
    setCount(nextValue);
  };

  return (
    <>
      <h1>oh hey triangle this is react, what's up??</h1>
      <div className="card">
        <button onClick={handleClick}>count from Rust is {count}</button>
      </div>
    </>
  );
}

export default App;
