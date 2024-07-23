import React, { useState } from 'react';
import NodeSelector from './components/NodeSelector';
import ReductionCalculator from './components/ReductionCalculator';

function App() {
  const [selectedNode, setSelectedNode] = useState(null);

  return (
    <div className="App">
      <header className="App-header">
        <h1>Node Selector</h1>
        <NodeSelector onSelectNode={setSelectedNode} />
        {selectedNode && <ReductionCalculator selectedNode={selectedNode} />}
      </header>
    </div>
  );
}

export default App;
