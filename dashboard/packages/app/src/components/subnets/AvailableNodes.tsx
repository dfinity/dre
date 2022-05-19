import React from 'react';
import { makeStyles, Theme } from '@material-ui/core/styles';
import Tabs from '@material-ui/core/Tabs';
import Tab from '@material-ui/core/Tab';
import Typography from '@material-ui/core/Typography';
import { Button, Chip, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, Divider, Grid } from '@material-ui/core';
import { Node, Operator, Host, NodeHealth } from './types';
import CheckCircleOutlineIcon from '@material-ui/icons/CheckCircleOutline';
import { green, blue, red, grey, lightBlue } from '@material-ui/core/colors';
import AddSubnetDialog from './AddSubnetDialog';
import ErrorOutlineSharpIcon from '@material-ui/icons/ErrorOutlineSharp';
import HelpOutlineOutlinedIcon from '@material-ui/icons/HelpOutlineOutlined';
import UpdateIcon from '@material-ui/icons/Update';
import HourglassEmptyIcon from '@material-ui/icons/HourglassEmpty';
import { fetchNodes, fetchOperators, fetchMissingHosts, fetchNodesHealths } from './fetch';
import { CodeSnippet } from '@backstage/core-components';
import _ from "lodash";

interface TabPanelProps {
  children?: React.ReactNode;
  index: any;
  value: any;
}

const useStyles = makeStyles((theme: Theme) => ({
  root: {
    flexGrow: 1,
    backgroundColor: theme.palette.background.paper,
    display: 'flex',
  },
  tabpanel: {
    height: '100%',
    marginTop: theme.spacing(1)
  },
  nodeChip: {
    margin: 5,
    fontFamily: "Roboto Mono"
  },
  tabs: {
    borderRight: `1px solid ${theme.palette.divider}`,
  },
  tooltip: {
    background: "none"
  },
  nodeChipsGroup: {
    paddingTop: theme.spacing(2),
  },
  nodeChipsDivider: {
    margin: theme.spacing(1),
  },
  nodeAvailabilityHeader: {
    marginLeft: theme.spacing(1),
  }
}));

const RemoveUnhealthyNodesDialog = ({ nodes }: { nodes: Node[] }) => {
  const [open, setOpen] = React.useState(false);

  const handleClickOpen = () => {
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
  };

  return (
    <React.Fragment>
      <Button color="primary" onClick={handleClickOpen}>
        Remove unhealthy
      </Button>
      <Dialog
        open={open}
        onClose={handleClose}
        aria-labelledby="max-width-dialog-title"
        maxWidth="xl"
      >
        <DialogTitle id="max-width-dialog-title">Remove unhealthy spare nodes</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Removing unhealthy nodes. Run the shell command below.
          </DialogContentText>
          <CodeSnippet text={`./mainnet-op propose-to-remove-nodes \\\n${nodes.sort((n1, n2) => n1.hostname.localeCompare(n2.hostname)).map(n => `${n.principal} \`# hostname: ${n.hostname}\``).join(" \\\n")}`} language="shell" showCopyCodeButton />
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose} color="primary">
            Close
          </Button>
        </DialogActions>
      </Dialog>
    </React.Fragment>
  )
}

type nodeAvailability = "Proposal pending" | NodeHealth;

function nodeAvailability(node: Node, health: NodeHealth): nodeAvailability {
  if (node.proposal) {
    return "Proposal pending";
  }
  return health;
}


type hostAvailability = "Free";

function hostAvailability(_: Host): hostAvailability {
  return "Free"
}

const NodeAvailabilityIcon = ({ availability }: { availability: nodeAvailability }) => {
  switch (availability) {
    case "Proposal pending":
      return <UpdateIcon style={{ color: blue[500] }} />
    case "Healthy":
      return <CheckCircleOutlineIcon style={{ color: green[500] }} />
    case "Unhealthy":
      return <ErrorOutlineSharpIcon style={{ color: red[500] }} />
    case "Unknown":
      return <HelpOutlineOutlinedIcon style={{ color: grey[500] }} />
  }
}

const HostAvailabilityIcon = ({ availability }: { availability: hostAvailability }) => {
  switch (availability) {
    case "Free":
      return <HourglassEmptyIcon style={{ color: lightBlue[500] }} />
  }
}

function TabPanel({
  children,
  value,
  index,
  unassignedNodes,
  missingHosts,
  operators,
  dc,
  ...other
}: TabPanelProps & { dc: string, unassignedNodes: Node[], missingHosts: Host[], operators: Operator[] }) {
  const classes = useStyles();

  const healths = fetchNodesHealths()

  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`vertical-tabpanel-${index}`}
      aria-labelledby={`vertical-tab-${index}`}
      {...other}
      className={classes.tabpanel}
    >
      {value === index && (
        <Grid container justifyContent="space-evenly" style={{ height: '100%' }}>
          <Grid item xs>
            <Grid container justifyContent="flex-start" alignItems="center">
              <Grid item>
                <Typography variant='h5'>Unassigned nodes</Typography>
                <Typography variant='subtitle1'>Ready to join a subnet</Typography>
                <Typography variant="subtitle2">Total: {unassignedNodes.length}</Typography>
                <Typography variant="subtitle2">
                  Assignable to a new subnet:
                  {" "}
                  {unassignedNodes.map(n => nodeAvailability(n, healths[n.principal] ?? "Unknown")).filter(a => a == "Healthy").length}
                </Typography>
              </Grid>
              <Grid item>
                <AddSubnetDialog />
                <RemoveUnhealthyNodesDialog nodes={unassignedNodes.filter(n => healths[n.principal] != "Healthy")} />
              </Grid>
            </Grid>
            {
              _(
                (dc === "all" ? unassignedNodes : unassignedNodes.filter(n => n.operator.datacenter.name === dc))
                  .sort((a, b) => a.hostname.localeCompare(b.hostname))
                  .map(n => ({ node: n, status: healths[n.principal] ?? "Unknown" }))
                  .map(({ node, status }) => ({ node: node, status: status!! })),
              ).groupBy(
                (s) => nodeAvailability(s.node, s.status),
              ).map((nodes, availability) =>
                <Grid className={classes.nodeChipsGroup}>
                  <Grid container>
                    <Grid>
                      <NodeAvailabilityIcon availability={availability as nodeAvailability} />
                    </Grid>
                    <Grid>
                      <Typography className={classes.nodeAvailabilityHeader}>
                        {availability} ({nodes.length})
                      </Typography>
                    </Grid>
                  </Grid>
                  <Divider className={classes.nodeChipsDivider} />
                  {nodes.map(({ node }) =>
                    <Chip
                      label={node.hostname}
                      variant="outlined"
                      size="small"
                      className={classes.nodeChip}
                    />
                  )}
                </Grid>
              ).value()
            }
          </Grid>
          <Divider orientation="vertical" variant="middle" />
          <Grid item xs>
            <Grid container justifyContent="flex-start" alignItems="center">
              <Grid item>
                <Typography variant='h5'>Missing hosts</Typography>
                <Typography variant='subtitle1'>Hosts not participating in the Mainnet network</Typography>
                <Typography variant="subtitle2">Total allowed: {operators.reduce((r, c) => r + c.allowance, 0)}</Typography>
              </Grid>
              <Grid item>
              </Grid>
            </Grid>
            {
              _(
                (dc === "all" ? missingHosts : missingHosts.filter(n => n.datacenter === dc))
                  .sort((a, b) => a.name.localeCompare(b.name))
              ).groupBy(
                (h) => hostAvailability(h),
              ).map((hosts, availability) =>
                <Grid className={classes.nodeChipsGroup}>
                  <Grid container>
                    <Grid>
                      <HostAvailabilityIcon availability={availability as hostAvailability} />
                    </Grid>
                    <Grid>
                      <Typography className={classes.nodeAvailabilityHeader}>
                        {availability} ({hosts.length})
                      </Typography>
                    </Grid>
                  </Grid>
                  <Divider className={classes.nodeChipsDivider} />
                  {hosts.map(host =>
                    <Chip
                      label={host.name}
                      variant="outlined"
                      size="small"
                      className={classes.nodeChip}
                    />
                  )}
                </Grid>
              ).value()
            }
          </Grid>
        </Grid>
      )}
    </div>
  );
}

function a11yProps(index: any) {
  return {
    id: `vertical-tab-${index}`,
    'aria-controls': `vertical-tabpanel-${index}`,
  };
}

export const AvailableNodes = () => {
  const classes = useStyles();
  const [value, setValue] = React.useState(0);

  const handleChange = (_: React.ChangeEvent<{}>, newValue: number) => {
    setValue(newValue);
  };
  const missingHosts = fetchMissingHosts();

  const nodes = fetchNodes();
  const operators = fetchOperators();

  const unassignedNodes = Object.values(nodes).filter(n => n.subnet === undefined);
  const missingHostsDatacenters = ["all", ...Array.from(new Set([...unassignedNodes.map(n => n.operator.datacenter.name), ...missingHosts.map(h => h.datacenter)])).sort()];

  return (
    <div className={classes.root}>
      <Grid container>
        <Grid item>
          <div className={classes.root}>
            <Tabs
              orientation="vertical"
              variant="scrollable"
              value={value}
              onChange={handleChange}
              aria-label="Vertical tabs example"
              className={classes.tabs}
            >
              {missingHostsDatacenters.map((dc, index) => (
                <Tab label={dc} {...a11yProps(index)} />
              ))}
            </Tabs>
          </div>
        </Grid>
        <Grid item xs>
          {missingHostsDatacenters.map((dc, index) => (
            <TabPanel
              value={value}
              index={index}
              dc={dc}
              unassignedNodes={unassignedNodes}
              missingHosts={missingHosts}
              operators={Object.values(operators)}
            />
          ))}
        </Grid>
      </Grid>
    </div>
  );
}
