import React, { useState, useEffect } from 'react';
import { Box, Autocomplete, TextField, TableCell, TableRow, TableHead, Table, TableContainer, Paper, TableBody } from '@mui/material';
import { Link } from 'react-router-dom';
import { styled } from '@mui/material/styles';
import { PeriodFilter } from './FilterBar';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';

export const Root = styled('div')(({ theme }) => ({
  width: '100%',
  ...theme.typography.body2,
  color: theme.palette.text.secondary,
  '& > :not(style) ~ :not(style)': {
    marginTop: theme.spacing(2),
  },
}));

export interface NodeListProps {
  nodeRewards: NodeRewardsResponse[];
  periodFilter: PeriodFilter;
}

const tableHeaders = [
  { label: 'Node ID', key: 'node_id' },
  { label: 'Node Provider ID', key: 'node_provider_id' },
  { label: 'Days Assigned', key: 'days_assigned' },
  { label: 'Rewards Percent', key: 'rewards_percent' },
  { label: 'Failure Rate Avg.', key: 'failure_rate' },
];

export const NodeList: React.FC<NodeListProps> = ({ nodeRewards }) => {
  const [filteredMetrics, setFilteredMetrics] = useState<NodeRewardsResponse[]>(nodeRewards);

  useEffect(() => {
    setFilteredMetrics(nodeRewards);
  }, [nodeRewards]);

  const handleSearchChange = (event: React.SyntheticEvent, value: string | null) => {
    if (value) {
      setFilteredMetrics(nodeRewards.filter(node => node.node_id.toText().includes(value)));
    } else {
      setFilteredMetrics(nodeRewards);
    }
  };

  return (
    <>
      <Box sx={{ p: 3 }}>
        <Autocomplete
          freeSolo
          id="node-search"
          options={nodeRewards.map(node => node.node_id.toText())}
          onInputChange={handleSearchChange}
          renderInput={(params) => (
            <TextField {...params} label="Search Node" variant="outlined" />
          )}
        />
      </Box>
      <Box sx={{ m: 3 }}>
        <Paper sx={{ bgcolor: 'background.paper', borderRadius: '10px' }}>
          <TableContainer>
            <Table aria-label="node metrics table">
              <TableHead>
                <TableRow>
                  {tableHeaders.map(header => (
                    <TableCell key={header.key}>{header.label}</TableCell>
                  ))}
                </TableRow>
              </TableHead>
              <TableBody>
                {filteredMetrics.map(nodeMetrics => (
                  <TableRow
                    key={nodeMetrics.node_id.toText()}
                    sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
                  >
                    <TableCell component="th" scope="row">
                      <Link to={`/nodes/${nodeMetrics.node_id.toText()}`} className="custom-link">
                        {nodeMetrics.node_id.toText()}
                      </Link>
                    </TableCell>
                    <TableCell>{nodeMetrics.node_provider_id.toText()}</TableCell>
                    <TableCell>{nodeMetrics.daily_node_metrics.length}</TableCell>
                    <TableCell>{Math.round(nodeMetrics.rewards_percent * 100)}%</TableCell>
                    <TableCell>{Math.round(nodeMetrics.rewards_stats.failure_rate * 100)}%</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        </Paper>
      </Box>
    </>
  );
};
