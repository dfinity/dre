import React, { useState } from 'react';
import FilterBar, { Filters } from './components/FilterBar.js';
import { Box, CircularProgress, Drawer, List, ListItem, Menu, MenuItem, CssBaseline, ThemeProvider, createTheme, Typography, Toolbar, IconButton } from '@mui/material';
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js'; // Adjust the path as needed
import { SubnetNodeMetricsArgs, SubnetNodeMetricsResult } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.js';
import { NodeMetrics } from './models/NodeMetrics.js';
import ChartGrid from './components/ChartGrid.js';
import Header from './components/Header.js';
import Logo from './assets/icp_logo.svg'; // Import SVG as React component

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    divider: '#121212',
  },
});

function App() {
  const [filters, setFilters] = useState<Filters>({
    dateStart: new Date(),
    dateEnd: new Date(),
    subnet: '' 
  });
  const [data, setData] = useState<NodeMetrics[]>([]);
  const [filteredData, setFilteredData] = useState<NodeMetrics[]>([]);
  const [subnets, setSubnets] = useState<Set<string>>(new Set());
  const [error, setError] = useState("");
  const [drawerOpen, setDrawerOpen] = useState(false); // Add this state
  const [menuAnchorEl, setMenuAnchorEl] = useState<null | HTMLElement>(null); // Menu anchor state
  const drawerWidth = 120;

  React.useEffect(() => {
    const fetchNodes = async () => {
      try {
        const request: SubnetNodeMetricsArgs = {
          ts: [],
          subnet_id: [],
        };
        const response: SubnetNodeMetricsResult = await trustworthy_node_metrics.subnet_node_metrics(request);

        console.info("Got response", response);

        if ('Ok' in response) {
          const metrics: NodeMetrics[] = response.Ok.flatMap((metricResponse) => {
            return metricResponse.node_metrics.map((nodeMetrics) => {
              return new NodeMetrics(
                metricResponse.ts,
                nodeMetrics.num_block_failures_total,
                nodeMetrics.node_id,
                nodeMetrics.num_blocks_proposed_total,
                metricResponse.subnet_id
              );
            })
          });
          const subnets: Set<string> = new Set(metrics.map(metric => metric.subnet_id.toText()));

          setData(metrics);
          setSubnets(subnets);
        } else {
          setError(response.Err);
        }
      } catch (error) {
        console.error("Error fetching nodes:", error);
      }
    };

    fetchNodes();
  }, []);

  React.useEffect(() => {
    const filterData = () => {
      const f = data.filter((metrics) => {
        const metricsDate = metrics.date; 
        const isDateInRange = metricsDate >= filters.dateStart && metricsDate <= filters.dateEnd;

        return metrics.subnet_id.toText() === filters.subnet && isDateInRange;
      });

      setFilteredData(f);
    }

    filterData();
  }, [filters, data]);

  const handleMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setMenuAnchorEl(event.currentTarget);
  };

  const handleMenuClose = () => {
    setMenuAnchorEl(null);
  };

  const handleSubnetSelect = (subnet: string) => {
    setFilters((prev) => ({ ...prev, subnet }));
    handleMenuClose();
  };

  if (error) {
    return <div>Error: {error}</div>;
  }

  if (data.length === 0) {
    return <CircularProgress />;
  }

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Box sx={{ display: 'flex' }}>
        <Drawer
          variant="permanent"
          open={drawerOpen}
          onClose={() => setDrawerOpen(false)}
          sx={{ width: drawerWidth, flexShrink: 0 }}
          PaperProps={{
            sx: {
              width: drawerWidth,
              bgcolor: 'palette.background.paper', 
            }
          }}
        >
          <List>
            <ListItem>
              <IconButton edge="start" color="inherit" aria-label="logo">
                <img src={Logo} alt="Logo" style={{ height: 30 }} />
              </IconButton>
            </ListItem>
            <ListItem>
              <Toolbar />
            </ListItem>
            <ListItem button onClick={handleMenuOpen}>
              <Typography variant="h6">Subnets</Typography>
            </ListItem>
          </List>
        </Drawer>
        <Box
          component="main"
          sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)` }}
        >
          <Header />
          <Box sx={{ mb: 2 }}>
            <FilterBar filters={filters} setFilters={setFilters} subnets={subnets} />
          </Box>
          <ChartGrid data={filteredData} />
        </Box>
      </Box>
      <Menu
        anchorEl={menuAnchorEl}
        open={Boolean(menuAnchorEl)}
        onClose={handleMenuClose}
      >
        <MenuItem value="" onClick={() => handleSubnetSelect('')}>
          <em>None</em>
        </MenuItem>
        {Array.from(subnets).map((subnet, index) => (
          <MenuItem  key={index} onClick={() => handleSubnetSelect(subnet)}>
            {subnet}
          </MenuItem>
        ))}
      </Menu>
    </ThemeProvider>
  );
}

export default App;
