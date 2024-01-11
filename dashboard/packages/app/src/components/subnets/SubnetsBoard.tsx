import React from 'react';
import { makeStyles, Theme } from '@material-ui/core/styles';
import Card from '@material-ui/core/Card';
import CardActions from '@material-ui/core/CardActions';
import CardContent from '@material-ui/core/CardContent';
import Button from '@material-ui/core/Button';
import Typography from '@material-ui/core/Typography';
import { Avatar, Chip, Dialog, DialogTitle, DialogActions, DialogContent, Grid, DialogContentText, IconButton } from '@material-ui/core';
import FileCopyOutlinedIcon from '@material-ui/icons/FileCopyOutlined';
import ReportSharpIcon from '@material-ui/icons/ReportSharp';
import ReportProblemSharpIcon from '@material-ui/icons/ReportProblemSharp';
import InfoIcon from '@material-ui/icons/Info';
import { lightBlue, orange, red } from '@material-ui/core/colors';
import { NodeHealth, Subnet, VerifiedApplication } from './types';
import { useApi, configApiRef } from '@backstage/core-plugin-api';

import { fetchSubnets, fetchNodesHealths, get_network } from './fetch';

const useStyles = makeStyles((theme: Theme) => ({
  root: {
    '& > :nth-child(n+2)': {
      marginTop: theme.spacing(1),
    }
  },
  card: {
    height: '100%',
  },
  compactCard: {
    '& > div': {
      paddingTop: theme.spacing(1),
      paddingBottom: theme.spacing(1),
    }
  },
  divider: {
    margin: theme.spacing(2),
  },
}));

export const verifiedApplicationLogo = (va: VerifiedApplication) => {
  switch (va) {
    case "Fleek":
      return "/fleek.jpeg"
    case "Distrikt":
      return "/distrikt.jpeg"
    default:
      return undefined
  }
}

type ActionUrgency = "critical" | "warning" | "info";

function ActionIcon({ action }: { action: Action }) {
  switch (action.urgency) {
    case "critical":
      return <ReportSharpIcon style={{ color: red[500] }} />
    case "warning":
      return <ReportProblemSharpIcon style={{ color: orange[500] }} />
    case "info":
      return <InfoIcon style={{ color: lightBlue[500] }} />
  }
}

function actionTypeButtonText(actionType: ActionType) {
  switch (actionType) {
    case 'expand':
      return "Expand"
    case 'heal':
      return "Heal"
    case 'replace':
      return "View"
  }
}

function ActionDialog({ subnet, action }: { subnet: Subnet, action: Action }) {
  const [open, setOpen] = React.useState(false);

  const handleClickOpen = () => {
    if (action.url) {
      let tab = window.open(action.url, '_blank');
      if (tab) {
        tab.focus();
      }
    } else {
      setOpen(true);
    }
  };

  const handleClose = () => {
    setOpen(false);
  };
  return (
    <React.Fragment>
      <Button color="primary" onClick={handleClickOpen}>
        {actionTypeButtonText(action.type)}
      </Button>
      <Dialog
        open={open}
        onClose={handleClose}
        aria-labelledby="max-width-dialog-title"
      >
        <DialogTitle id="max-width-dialog-title">{action.type[0].toUpperCase() + action.type.substring(1)} {subnet.metadata.name} Subnet ({subnet.principal.split("-")[0]})</DialogTitle>
        <DialogContent>
          <DialogContentText>
            $ {action.description}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <IconButton
            color="default"
            aria-label="copy"
            component="span"
            size="small"
            onClick={() => { navigator.clipboard.writeText(action.description || "") }}
          >
            <FileCopyOutlinedIcon fontSize="inherit" />
          </IconButton>
          <Button onClick={handleClose} color="secondary">
            Close
          </Button>
        </DialogActions>
      </Dialog>
    </React.Fragment>
  );
}

type ActionType = "expand" | "heal" | "replace"

interface Action {
  urgency: ActionUrgency
  message: React.ReactNode
  type: ActionType
  description?: string
  url?: string
}

// TODO: move to Rust backend
function generateSubnetActions(subnet: Subnet, healths: { [principal: string]: NodeHealth }): Action[] {
  let actions = new Array<Action>();
  const minimumSubnetSize = 13;
  if (subnet.nodes.length < minimumSubnetSize) {
    actions.push({
      type: "expand",
      urgency: "warning",
      description: `Guests will be added to the subnet: TODO`,
      message: <>Subnet should be extended with <b>{minimumSubnetSize - subnet.nodes.length}</b> more node{minimumSubnetSize - subnet.nodes.length > 1 && "s"}.</>,
    })
  }
  let deadNodes = subnet.nodes.filter(n => healths[n.principal] === "Dead");
  if (deadNodes.length > 0) {
    actions.push({
      type: "heal",
      urgency: deadNodes.length / subnet.nodes.length > 0.1 ? "critical" : "warning",
      description: `dre subnet --id ${subnet.principal} replace  # dead nodes: ${deadNodes.map(dn => dn.principal.split('-')[0] + "/" + dn.hostname)}`,
      message: <>There {deadNodes.length === 1 ? "is" : "are"} <b>{deadNodes.length}</b> dead node{deadNodes.length > 1 && "s"} that need{deadNodes.length === 1 && "s"} to be replaced.</>,
    })
  }
  let degraded = subnet.nodes.filter(n => healths[n.principal] === "Degraded");
  if (degraded.length > 0) {
    actions.push({
      type: "heal",
      urgency: degraded.length / subnet.nodes.length > 0.2 ? "critical" : "warning",
      description: `dre subnet --id ${subnet.principal} replace  # degraded nodes: ${degraded.map(dn => dn.principal.split('-')[0] + "/" + dn.hostname)}`,
      message: <>There {degraded.length === 1 ? "is" : "are"} <b>{degraded.length}</b> degraded node{degraded.length > 1 && "s"} that might need to be replaced.</>,
    })
  }
  if (subnet.proposal) {
    actions.push({
      type: "replace",
      urgency: "info",
      message: "Subnet has pending replacement proposal",
      url: `${useApi(configApiRef).getString('app.baseUrl')}/network/${get_network()}/subnet/${subnet.principal}/change`,
    })
  }

  return actions;
}

function SubnetCard({ subnet, actions, compact }: { subnet: Subnet, actions?: Action[], compact?: boolean }) {
  compact = compact || false;
  const classes = useStyles();

  return (
    <Card raised={!compact} className={`${classes.card} ${compact && classes.compactCard}`}>
      <CardContent>
        <Typography gutterBottom={!compact} variant={compact ? "h6" : "h5"} component="h3">
          {subnet.metadata.name}
          <Typography color="textSecondary" style={{ fontWeight: "normal", display: "inline" }}>
            {" ( "}
            {subnet.principal.split('-')[0]}
            <IconButton
              color="default"
              aria-label="copy"
              component="span"
              size="small"
              onClick={() => { navigator.clipboard.writeText(subnet.principal) }}
            >
              <FileCopyOutlinedIcon fontSize="inherit" />
            </IconButton>
            )
          </Typography>
        </Typography>
        <Typography color="textSecondary" gutterBottom={!compact} variant={compact ? "subtitle2" : "subtitle1"}>
          {subnet.subnet_type.split("_").map(w => w[0].toUpperCase() + w.substring(1)).join(" ")}
        </Typography>
        {subnet.metadata.labels?.map(l => <Chip label={l} size="small" />)}
        {subnet.metadata.applications?.map(a => <Chip avatar={<Avatar src={verifiedApplicationLogo(a)} />} label={a} size="small" />)}
        {actions && actions.length > 0 && actions.map(action => (
          <CardActions>
            <Grid container justifyContent="center">
              <Grid item>
                <ActionIcon action={action} />
              </Grid>
              <Grid item xs>
                <Typography variant="body2">{action.message}</Typography>
              </Grid>
              <Grid item>
                <ActionDialog subnet={subnet} action={action} />
              </Grid>
            </Grid>
          </CardActions>
        ))}
      </CardContent>
    </Card>
  );
}

export default function SubnetsBoard() {
  const classes = useStyles();
  const subnets = fetchSubnets();

  const healths = fetchNodesHealths();
  const subnetsWithActions = Object.values(subnets).map(s => {
    let subnetActions = generateSubnetActions(s, healths);
    return {
      ...s,
      ...{ actions: subnetActions }
    };
  }).sort((s1, s2) => {
    if (s1.metadata.name === "NNS") {
      return -1
    } else if (s2.metadata.name === "NNS") {
      return 1
    }

    if (s1.metadata.name === "People Parties") {
      return -1
    } else if (s2.metadata.name === "People Parties") {
      return 1
    }

    if (s1.subnet_type === "system") {
      return -1
    } else if (s2.subnet_type === "system") {
      return 1
    }

    return parseInt(s1.metadata.name.split(" ")[1]) - parseInt(s2.metadata.name.split(" ")[1])
  });

  return (
    <div className={classes.root}>
      <Grid container alignItems="stretch">
        {subnetsWithActions.filter(s => s.actions.length > 0).map((subnet) => (
          <Grid item xs={2}>
            <SubnetCard subnet={subnet} actions={subnet.actions} />
          </Grid>
        ))}
      </Grid>
      <Grid container alignItems="stretch" spacing={1}>
        {subnetsWithActions.filter(s => s.actions.length == 0).map((subnet) => (
          <Grid item xs={2}>
            <SubnetCard subnet={subnet} compact />
          </Grid>
        ))}
      </Grid>
    </div>
  )
}
