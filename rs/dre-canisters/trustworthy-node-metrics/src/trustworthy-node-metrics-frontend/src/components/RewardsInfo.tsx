import React from 'react';
import { Grid, List, ListItem, Typography } from '@mui/material';
import 'katex/dist/katex.min.css';
import { InlineMath } from 'react-katex';
import { axisClasses, ChartsReferenceLine, LineChart } from '@mui/x-charts';

const NodeRewardExplanation = () => {
  return (
    <Grid container>
      {/* Title Section */}
      <Grid item xs={12}>
        <Typography variant="body1" gutterBottom>
          How are rewards computed?
        </Typography>
      </Grid>
      <Grid item xs={12} md={4}>
        <Typography variant="body2" gutterBottom>
          Node Unassigned:
        </Typography>
        <Typography variant="body2" color="textSecondary" gutterBottom>
          When a node is not assigned to any subnet, it automatically receives the full reward (100%).
        </Typography>

        {/* Node Assigned Section */}
        <Typography variant="body2" gutterBottom>
          Node Assigned:
        </Typography>
        <Typography variant="body2" color="textSecondary" gutterBottom>
          When assigned to a subnet, the reward calculation follows these steps:
        </Typography>

        {/* Daily Block Data Collection */}
        <List sx={{ listStyle: 'circle', ml: 4 }}>
          <ListItem sx={{ display: 'list-item' }}>
            <Typography variant="body2" gutterBottom>
              Daily Block Data Collection:
            </Typography>
            <Typography variant="body2" color="textSecondary" gutterBottom>
              - Blocks Proposed: The number of blocks the node successfully proposes.
            </Typography>
            <Typography variant="body2" color="textSecondary" gutterBottom>
              - Blocks Failed: The number of blocks the node fails to propose.
            </Typography>
          </ListItem>
        </List>

        {/* Failure Rate Calculation */}
        <List sx={{ listStyle: 'circle', ml: 4 }}>
          <ListItem sx={{ display: 'list-item' }}>
            <Typography variant="body2" gutterBottom>
              Failure Rate Calculation:
            </Typography>
            <Typography variant="body2" color="textSecondary" gutterBottom>
              We compute the failure rate using the following formula:
            </Typography>
            <Typography variant="body2" gutterBottom>
              <InlineMath math="Failure \, Rate = \frac{\text{Blocks Failed Total}}{\text{Blocks Proposed Total} + \text{Blocks Failed Total}}" />
            </Typography>
            <Typography variant="body2" color="textSecondary" gutterBottom>
              This gives the proportion of blocks the node failed to produce relative to the total expected in a given month.
            </Typography>
          </ListItem>
        </List>
        </Grid>
        <Grid item xs={12} md={4}>

        {/* Linear Reduction Function */}
        <List sx={{ listStyle: 'circle', ml: 4 }}>
          <ListItem sx={{ display: 'list-item' }}>
            <Typography variant="body2" gutterBottom>
              Apply Linear Reduction Function:
            </Typography>
            <Typography variant="body2" color="textSecondary" gutterBottom>
              Based on the failure rate, we apply a linear reduction function to determine how much the failure rate reduces the node's rewards.
            </Typography>

            {/* Specific Failure Rate Conditions */}
            <List sx={{ listStyle: 'circle', ml: 4 }}>
              <ListItem sx={{ display: 'list-item' }}>
                <Typography variant="body2" color="textSecondary" gutterBottom>
                  Failure Rates Below 10%: For failure rates ≤ 10%, there is no reduction in rewards. The rewards reduction is 0%, meaning for performance below this threshold, rewards remain unaffected.
                </Typography>
              </ListItem>
              <ListItem sx={{ display: 'list-item' }}>
                <Typography variant="body2" color="textSecondary" gutterBottom>
                  Failure Rates Above 80%: Once the failure rate exceeds 80%, the rewards reduction reaches its maximum of 100%. Any failure rate beyond this threshold results in a complete loss of rewards.
                </Typography>
              </ListItem>
            </List>

            <Typography variant="body2" color="textSecondary" gutterBottom>
              The reward multiplier for the assigned period is computed by subtracting the rewards reduction from 100%.
            </Typography>
          </ListItem>
        </List>
      </Grid>
        <Grid item xs={12} md={4}>
          <List sx={{ listStyle: 'circle', ml: 4 }}>
          {/* Total Rewards Calculation Placeholder */}
          <ListItem sx={{ display: 'list-item' }}>
            <Typography variant="body2" gutterBottom>
              Compute Reward Multiplier:
            </Typography>
            <Typography variant="body2" color="textSecondary" gutterBottom>
              The final reward multiplier is then the weighted average between the multiplier for days in which the node is assigned to a subnet and 100% for the days in which the node is unassigned. 
            </Typography>
          </ListItem>
        </List>
        </Grid>
    </Grid>
  );
};


export default NodeRewardExplanation;

export const LinearReductionChart: React.FC<{ failureRate: number; rewardReduction: number }> = ({ failureRate, rewardReduction }) => {
  const MIN_FAILURE_RATE = 10;
  const MAX_FAILURE_RATE = 80;

  // Create dataset for chart
  const dataset = Array.from({ length: 101 }, (_, index) => {
    const rewardsRatePercent = index < MIN_FAILURE_RATE ? 0 :
      index > MAX_FAILURE_RATE ? 100 :
      ((index - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)) * 100;

    const dotPoints = index === failureRate ? rewardsRatePercent : null;

    return { failureRatePercent: index, rewardsRatePercent, dotPoints };
  });

  return (
    <>
      <LineChart
        margin={{ left: 60}}
        yAxis={[{
          label: 'Rewards reduction',
          valueFormatter: (value: number) => `${value}%`,
        }]}
        xAxis={[{
          dataKey: 'failureRatePercent',
          label: 'Failure rate',
          colorMap: {
            type: 'piecewise',
            thresholds: [failureRate],
            colors: ['#FF6347', '#4CAF79'],
          },
          valueFormatter: (value: number) => `${value}%`,
        }]}
        series={[
          { dataKey: 'rewardsRatePercent', showMark: false},
          { dataKey: 'dotPoints' },
        ]}
        tooltip={{ trigger: 'none' }} 
        dataset={dataset}
        height={300}
        sx={{
          [`.${axisClasses.left} .${axisClasses.label}`]: {
            transform: 'translate(-20px, 0)',
          },
        }}
      >
        <ChartsReferenceLine
          x={failureRate}
          label={`Failure Rate: ${failureRate}%`}
          lineStyle={{ stroke: 'white' }}
        />
        <ChartsReferenceLine
          y={rewardReduction}
          label={`Rewards Reduction: ${rewardReduction}%`}
          lineStyle={{ stroke: 'white' }}
        />
      </LineChart>
    </>
  );
};
