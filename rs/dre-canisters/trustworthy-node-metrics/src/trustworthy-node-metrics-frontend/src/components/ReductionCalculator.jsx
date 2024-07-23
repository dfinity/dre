import React, { useState } from 'react';
import { Actor, HttpAgent } from '@dfinity/agent';
import { idlFactory } from '../../../declarations/trustworthy-node-metrics'; // Adjust the path as needed

const ReductionCalculator = ({ selectedNode }) => {
  const [rate, setRate] = useState(1.0);
  const [consecutiveDays, setConsecutiveDays] = useState(false);
  const [reduction, setReduction] = useState(null);

  const calculateReduction = async () => {
    const agent = new HttpAgent({ host: 'http://localhost:8000' });
    const actor = Actor.createActor(idlFactory, { agent, canisterId: 'your-canister-id' }); // Replace 'your-canister-id' with actual ID
    const penalty = {
      consecutive_days: consecutiveDays,
      rate: parseFloat(rate)
    };
    const reductionValue = await actor.calculate_reduction_for_node(selectedNode, penalty);
    setReduction(reductionValue);
  };

  return (
    <div>
      <div>
        <label>
          Rate:
          <input
            type="number"
            value={rate}
            onChange={(e) => setRate(e.target.value)}
          />
        </label>
      </div>
      <div>
        <label>
          Consecutive Days:
          <input
            type="checkbox"
            checked={consecutiveDays}
            onChange={(e) => setConsecutiveDays(e.target.checked)}
          />
        </label>
      </div>
      <button onClick={calculateReduction}>Calculate Reduction</button>
      {reduction !== null && <p>Reduction: {reduction}</p>}
    </div>
  );
};

export default ReductionCalculator;
