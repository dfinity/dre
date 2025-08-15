import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { SxProps, Theme } from '@mui/material';
import { NodeRewardsMultiplier } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';

interface RewardTableProps {
    nodeRewards: NodeRewardsMultiplier[],
    sx?: SxProps<Theme>;
  }

const RewardTable: React.FC<RewardTableProps> = ({ nodeRewards }) => {
  return (
    <TableContainer component={Paper}>
      <Table aria-label="simple table">
        <TableHead>
          <TableRow>
            <TableCell>Nodes</TableCell>
            <TableCell>Failure Rates Avg.</TableCell>
            <TableCell>Rewards No Penalty</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {nodeRewards.map((nodeMetrics) => (
            <TableRow
              key={nodeMetrics.node_id.toText()}
              sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
            >
              <TableCell component="th" scope="row">
                {nodeMetrics.node_id.toText()}
              </TableCell>
              <TableCell component="th" scope="row">
                {nodeMetrics.rewards_multiplier_stats.failure_rate * 100}%
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
}

export default RewardTable;
