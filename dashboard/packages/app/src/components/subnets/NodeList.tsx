import React from 'react';
import { makeStyles, Chip, Box, Typography, Theme } from '@material-ui/core';
import { green, blue, deepOrange } from '@material-ui/core/colors';
import CheckCircleOutlineIcon from '@material-ui/icons/CheckCircleOutline';
import HelpOutlineIcon from '@material-ui/icons/HelpOutline';
import PublishIcon from '@material-ui/icons/Publish';
import { Guest } from './types';

import AppBar from '@material-ui/core/AppBar';
import Tabs from '@material-ui/core/Tabs';
import Tab from '@material-ui/core/Tab';

type GuestState = "healthy" | "unknown" | "ready";

const stateIcon = (state: GuestState) => {
  switch (state) {
    case "healthy":
      return <CheckCircleOutlineIcon style={{ color: green[500] }} />
    case "ready":
      return <CheckCircleOutlineIcon style={{ color: blue[500] }} />
    case "unknown":
      return <HelpOutlineIcon style={{ color: deepOrange[500] }} />
  }
}

interface TabPanelProps {
  children?: React.ReactNode;
  index: any;
  value: any;
}

function TabPanel(props: TabPanelProps) {
  const { children, value, index, ...other } = props;

  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`scrollable-auto-tabpanel-${index}`}
      aria-labelledby={`scrollable-auto-tab-${index}`}
      style={{ maxHeight: 200, overflowY: 'scroll' }}
      {...other}
    >
      {value === index && (
        <Box p={3}>
          <Typography>{children}</Typography>
        </Box>
      )}
    </div>
  );
}

function a11yProps(index: any) {
  return {
    id: `scrollable-auto-tab-${index}`,
    'aria-controls': `scrollable-auto-tabpanel-${index}`,
  };
}

const useStyles = makeStyles((theme: Theme) => ({
  root: {
    flexGrow: 1,
    width: '100%',
    backgroundColor: theme.palette.background.paper,
  },
  nodeChip: {
    margin: 5,
    fontFamily: "Roboto Mono"
  },
}));

export const NodeList = ({ guests, state, move }: { guests: Guest[], state: GuestState, move?: (g: Guest) => void }) => {
  const classes = useStyles();
  const [value, setValue] = React.useState(0);

  const handleChange = (_: React.ChangeEvent<{}>, newValue: number) => {
    setValue(newValue);
  };

  let guestsGrouped = guests.reduce((r: { dc: string, guests: Guest[] }[], g) => {
    r.find(e => e.dc == g.datacenter)?.guests?.push(g) || r.push({ dc: g.datacenter, guests: [g] });
    return r
  }, []).sort((a, b) => a.dc.localeCompare(b.dc));
  guestsGrouped = [{ dc: "all", guests: guests }, ...guestsGrouped]

  return (
    <div className={classes.root}>
      <AppBar position="static" color="default">
        <Tabs
          value={value}
          onChange={handleChange}
          indicatorColor="primary"
          textColor="primary"
          variant="scrollable"
          scrollButtons="auto"
          aria-label="scrollable auto tabs example"
        >
          {guestsGrouped.map(({ dc }, index) => <Tab label={dc} {...a11yProps(index)} />)}
        </Tabs>
      </AppBar>
      {guestsGrouped.map(({ guests }, index) =>
        <TabPanel value={value} index={index}>
          {guests.sort((a, b) => a.name.localeCompare(b.name)).map((guest: Guest) =>
            <Chip
              icon={stateIcon(state)}
              label={guest?.name}
              variant="outlined"
              size="small"
              {...(move ? {
                onDelete: () => move(guest),
                deleteIcon: <PublishIcon />,
              } : {})}
              className={classes.nodeChip}
            />
          )}
        </TabPanel>
      )}
    </div>
  );
}
