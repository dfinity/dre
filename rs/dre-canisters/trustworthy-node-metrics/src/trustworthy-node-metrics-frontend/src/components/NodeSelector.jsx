import React, { useState, useEffect } from 'react';
import { Actor, HttpAgent } from '@dfinity/agent';
import { trustworthy_node_metrics } from '../../../declarations/trustworthy-node-metrics'; // Adjust the path as needed

const NodeSelector = ({ onSelectNode }) => {
  const [nodes, setNodes] = useState([]);
  const [selectedNode, setSelectedNode] = useState('');

  useEffect(() => {
    const fetchNodes = async () => {
      try {
        const fetchedNodes = await trustworthy_node_metrics.get_nodes();
        setNodes(fetchedNodes);
      } catch (error) {
        console.error("Error fetching nodes:", error);
      }
    };

    fetchNodes();
  }, []);

  const handleChange = (event) => {
    const nodeId = event.target.value;
    setSelectedNode(nodeId);
    onSelectNode(nodeId);
  };

  return (
    <div>
      <label htmlFor="node-selector">Select a Node:</label>
      <select id="node-selector" value={selectedNode} onChange={handleChange}>
        <option value="" disabled>Select a node</option>
        {nodes.map((node) => (
          <option key={node.id} value={node.id}>
            {node.name}
          </option>
        ))}
      </select>
    </div>
  );
};

export default NodeSelector;
