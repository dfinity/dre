import React, { useState, useEffect } from 'react';
import { Box, Autocomplete, TextField, TableCell, TableRow, TableHead, Table, TableContainer, Paper, TableBody } from '@mui/material';
import { Link } from 'react-router-dom';
import { styled } from '@mui/material/styles';
import { NodeMetadata } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';

export const Root = styled('div')(({ theme }) => ({
  width: '100%',
  ...theme.typography.body2,
  color: theme.palette.text.secondary,
  '& > :not(style) ~ :not(style)': {
    marginTop: theme.spacing(2),
  },
}));

export interface NodeListProps {
  nodeProviderMapping: NodeMetadata[];
}

const tableHeaders = [
  { label: 'Node ID', key: 'node_id' },
  { label: 'Node Provider ID', key: 'node_provider_id' },
];

export const NodeList: React.FC<NodeListProps> = ({ nodeProviderMapping }) => {
  const [filteredNodes, setFilteredNodes] = useState<NodeMetadata[]>(nodeProviderMapping);

  useEffect(() => {
    setFilteredNodes(nodeProviderMapping);
  }, [nodeProviderMapping]);

  const handleSearchChange = (event: React.SyntheticEvent, value: string | null) => {
    if (value) {
      setFilteredNodes(nodeProviderMapping.filter(node => node.node_id.toText().includes(value)));
    } else {
      setFilteredNodes(nodeProviderMapping);
    }
  };

  return (
    <>
      <Box sx={{ p: 3 }}>
        <Autocomplete
          freeSolo
          id="node-search"
          options={nodeProviderMapping.map(node => node.node_id.toText())}
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
                {filteredNodes.map(nodesMap => (
                  <TableRow
                    key={nodesMap.node_id.toText()}
                    sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
                  >
                    <TableCell component="th" scope="row">
                      <Link to={`/nodes/${nodesMap.node_id.toText()}`} className="custom-link">
                        {nodesMap.node_id.toText()}
                      </Link>
                    </TableCell>
                    <TableCell>{nodesMap.node_provider_id.toText()}</TableCell>
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
