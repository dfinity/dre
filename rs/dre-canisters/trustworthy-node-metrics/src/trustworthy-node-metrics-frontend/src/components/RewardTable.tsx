import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { DashboardNodeRewards } from '../models/NodeMetrics';
import { SxProps, Theme } from '@mui/material';

interface RewardTableProps {
    dashboardNodeMetrics: DashboardNodeRewards[],
    sx?: SxProps<Theme>;
  }

const RewardTable: React.FC<RewardTableProps> = ({ dashboardNodeMetrics }) => {
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
          {dashboardNodeMetrics.map((nodeMetrics) => (
            <TableRow
              key={nodeMetrics.nodeId.toText()}
              sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
            >
              <TableCell component="th" scope="row">
                {nodeMetrics.nodeId.toText()}
              </TableCell>
              <TableCell component="th" scope="row">
                {Math.round(nodeMetrics.failureRateAvg * 100)}%
              </TableCell>
              <TableCell component="th" scope="row">
                {Math.round(nodeMetrics.rewardsNoPenalty * 100)}%
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
}

export default RewardTable;
