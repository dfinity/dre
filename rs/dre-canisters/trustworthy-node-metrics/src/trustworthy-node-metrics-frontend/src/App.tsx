import React, { useState, useEffect, useMemo } from 'react';
import { Alert, Box, CssBaseline, Snackbar, ThemeProvider, createTheme, useMediaQuery, useTheme } from '@mui/material';
import Drawer from './components/Drawer'; 
import { BrowserRouter as Router, Route, Routes, Navigate } from 'react-router-dom';
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js';
import { NodeList } from './components/NodeList';
import Header from './components/Header';
import { LoadingIndicator } from './utils/utils';
import { NodePage } from './components/NodePage';
import { NodeProviderPage } from './components/NodeProviderPage';
import { NodeMetadata } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.d';

// Theme configuration
const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#3f51b5',
    },
    secondary: {
      main: '#f50057',
    },
  },
});

const App: React.FC = () => {
  const [infoBanner, setInfoBanner] = useState<boolean | null>(true);
  const [providers, setProviders] = useState<Set<string>>(new Set());
  const [nodeMetadata, setNodeMetadata] = useState<NodeMetadata[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const theme = useTheme();
  const isSmallScreen = useMediaQuery(theme.breakpoints.down('sm'));
  const drawerWidth = 180;

  useEffect(() => {
    const updateRewards = async () => {
      try {
        setIsLoading(true);
        const nodeMetadata = await trustworthy_node_metrics.nodes_metadata();
        const providers = new Set(nodeMetadata.flatMap(node => node.node_provider_id.toText()));

        setProviders(providers);
        setNodeMetadata(nodeMetadata);
      } catch (error) {
        console.error("Error fetching nodeProviderMapping:", error);
      } finally {
        setIsLoading(false);
      }
    };
    
    updateRewards();
  }, []);

  const drawerProps = useMemo(() => ({
    providers,
    drawerWidth,
    temporary: isSmallScreen,
    drawerOpen,
    onClosed: () => setDrawerOpen(false)
  }), [providers, drawerWidth, isSmallScreen, drawerOpen]);

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Router>
        <Box sx={{ display: 'flex' }}>
          <Drawer {...drawerProps} />
          <Box sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)` }}>
            <Header withDrawerIcon={isSmallScreen} onDrawerIconClicked={() => setDrawerOpen(true)} />
            <Snackbar open={true} anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}>
              <Alert severity="warning" sx={{ width: '100%' }}>
                DEV dashboard
              </Alert>
            </Snackbar>
            <Snackbar open={!!infoBanner} onClose={() => setInfoBanner(null)} anchorOrigin={{ vertical: 'top', horizontal: 'center' }}>
              <Alert onClose={() => setInfoBanner(null)} severity="info" sx={{ width: '100%' }}>
                Only nodes that have been assigned to a subnet for at least one full day since 01/01/2024 are displayed
              </Alert>
            </Snackbar>
            <Routes>
              <Route path="/" element={<Navigate to="/nodes" replace />} />
              <Route path="/nodes" element={
                isLoading ? <LoadingIndicator /> : <NodeList nodeProviderMapping={nodeMetadata} />
              } />
              <Route path="/nodes/:node" element={ isLoading ? <LoadingIndicator /> : <NodePage nodeProvidersMapping={nodeMetadata} />} />
              <Route path="/providers/:provider" element={ isLoading ? <LoadingIndicator /> :  <NodeProviderPage  nodeMetadata={nodeMetadata} /> } />
            </Routes>
          </Box>
        </Box>
      </Router>
    </ThemeProvider>
  );
}

export default App;
