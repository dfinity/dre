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
  { label: 'Node Provider Name', key: 'node_provider_name' },
  { label: 'Data Center ID', key: 'dc_id' },
  { label: 'Region', key: 'region' },
];

export const NodeList: React.FC<NodeListProps> = ({ nodeProviderMapping }) => {
  const [filteredNodes, setFilteredNodes] = useState<NodeMetadata[]>(nodeProviderMapping);
  const [searchTerm, setSearchTerm] = useState<string>('');


  useEffect(() => {
    setFilteredNodes(nodeProviderMapping);
  }, [nodeProviderMapping]);

  const handleSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setSearchTerm(value);
    const lowerCaseValue = value.toLowerCase();

    if (value) {
      setFilteredNodes(nodeProviderMapping.filter(node =>
        node.node_metadata_stored.node_provider_name.join(' ').toLowerCase().includes(lowerCaseValue) ||
        node.node_id.toText().includes(lowerCaseValue) ||
        node.node_metadata_stored.node_provider_id.toText().includes(lowerCaseValue) ||
        node.node_metadata_stored.dc_id.toLowerCase().includes(lowerCaseValue) ||
        node.node_metadata_stored.region.toLowerCase().includes(lowerCaseValue) ||
        node.node_metadata_stored.node_type.toLowerCase().includes(lowerCaseValue)
      ));
    } else {
      setFilteredNodes(nodeProviderMapping);
    }
  };
  

  return (
    <>
      <Box sx={{ p: 3 }}>
      <TextField
          label="Search"
          variant="outlined"
          fullWidth
          value={searchTerm}
          onChange={handleSearchChange}
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
                    <TableCell>
                      <Link to={`/providers/${nodesMap.node_metadata_stored.node_provider_id.toText()}`} className="custom-link">
                        {nodesMap.node_metadata_stored.node_provider_id.toText()}
                      </Link>
                    </TableCell>
                    <TableCell>
                      <Link to={`/providers/${nodesMap.node_metadata_stored.node_provider_id.toText()}`} className="custom-link">
                        {nodesMap.node_metadata_stored.node_provider_name}
                      </Link>
                    </TableCell>
                    <TableCell>{nodesMap.node_metadata_stored.dc_id}</TableCell>
                    <TableCell>{nodesMap.node_metadata_stored.region}</TableCell>
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
