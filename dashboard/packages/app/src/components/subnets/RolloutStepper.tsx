import React from 'react';
import { makeStyles, Theme, createStyles } from '@material-ui/core/styles';
import { Stepper, Step, StepLabel, StepContent, Typography, Chip, Link, Grid, Divider, Paper, Tooltip } from '@material-ui/core';
import DoneIcon from '@material-ui/icons/Done';
import SyncIcon from '@material-ui/icons/Sync';
import HourglassEmptyIcon from '@material-ui/icons/HourglassEmpty';
import AvTimerIcon from '@material-ui/icons/AvTimer';
import HowToVoteIcon from '@material-ui/icons/HowToVote';
import HelpIcon from '@material-ui/icons/Help';
import RadioButtonCheckedIcon from '@material-ui/icons/RadioButtonChecked';
import RadioButtonUncheckedIcon from '@material-ui/icons/RadioButtonUnchecked';
import UpdateIcon from '@material-ui/icons/Update';
import { green, lightBlue, grey, purple, amber, orange, blue, red } from '@material-ui/core/colors';
import { Rollout, RolloutStage, SubnetUpdateState } from './types';
import { fetchRollouts } from './fetch';
import RolloutProgressStepper from './RolloutProgressStepper';
import _ from 'lodash';

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    root: {
      width: '100%',
      background: theme.palette.background.paper,
    },
    button: {
      marginTop: theme.spacing(1),
      marginRight: theme.spacing(1),
    },
    actionsContainer: {
      marginBottom: theme.spacing(2),
    },
    resetContainer: {
      padding: theme.spacing(3),
    },
    successIcon: {
      color: green[500],
    },
    bakeIcon: {
      color: orange[500],
    },
    proposalPendingIcon: {
      color: purple[500],
    },
    preparing: {
      color: lightBlue[500],
    },
    updatingIcon: {
      color: blue[500],
      animation: "$spin 4s linear infinite",
    },
    pauseIcon: {
      color: grey[500],
    },
    unknownIcon: {
      color: red[500],
    },
    versionChip: {
      '& > span': {
        lineHeight: "24px",
      },
      overflow: 'hidden',
    },
    rolloutHeader: {
      margin: theme.spacing(2),
    },
    releaseName: {
      fontFamily: "Roboto Mono",
    },
    "@keyframes spin": {
      "100%": {
        transform: "rotate(-360deg)",
      }
    },
    patchAvailableAvatar: {
      borderWidth: 2,
      borderStyle: "solid",
      borderColor: amber[500],
    },
    inactiveStep: {
      color: grey[500],
    },
    stepper: {
      overflowX: 'auto',
    },
    updateChip: {
      fontFamily: 'Roboto Mono',
      fontSize: '0.8em',
    }
  }),
);

const SubnetUpdateStateIcon = ({ state }: { state: SubnetUpdateState }) => {
  const classes = useStyles();
  switch (state) {
    case "scheduled":
      return <HourglassEmptyIcon className={classes.pauseIcon} />
    case "submitted":
      return <HowToVoteIcon className={classes.proposalPendingIcon} />
    case "preparing":
      return <UpdateIcon className={classes.preparing} />
    case "updating":
      return <SyncIcon className={classes.updatingIcon} />
    case "baking":
      return <AvTimerIcon className={classes.bakeIcon} />
    case "complete":
      return <DoneIcon className={classes.successIcon} />
    case "unknown":
      return <HelpIcon className={classes.unknownIcon} />
  }
}

const stateDescription = (state: SubnetUpdateState) => {
  switch (state) {
    case "scheduled":
      return "Subnet is scheduled to be updated at a later time."
    case "submitted":
      return "Proposal for the subnet update has been submitted."
    case "preparing":
      return "Proposal for the subnet update has been adopted and executed. The subnet is preparing a CUP so that it can be updated."
    case "updating":
      return "Subnet nodes are restarting to run the new version of the replica."
    case "baking":
      return "Functionality of the subnet is being verified by checking that no alerts are triggered in the next 30 minutes."
    case "complete":
      return "Subnet update is complete."
    case "unknown":
      return "Subnet update is in an unknown state."
  }
}


const RolloutStageContent = ({ stage }: { stage: RolloutStage }) => {
  const classes = useStyles();
  return (
    <Grid container direction='column' spacing={0}>
      {stage.updates.map(update => <Grid item justifyContent='center'>
        <Tooltip interactive title={
          <div style={{ maxWidth: 150 }}>
            <Typography display="block" variant="caption">{`${update.state[0].toUpperCase()}${update.state.substring(1)}`}</Typography>
            <Typography display="block" variant="caption" style={{ fontStyle: "italic", fontSize: "0.6rem" }} >
              {stateDescription(update.state)}
            </Typography>
          </div>
        } placement="left">
          <Chip
            size="small"
            label={`${update.subnet_id.split('-')[0]} (${update.subnet_name})`}
            onClick={() => window.open(`https://dashboard.internetcomputer.org/proposal/${update.proposal?.info?.id}`)}
            icon={<SubnetUpdateStateIcon state={update.state} />}
            disabled={!stage.active}
            variant={update.state == "scheduled" && "outlined" || "default"}
            className={classes.updateChip}
          />
        </Tooltip>
      </Grid>)
      }
    </Grid >
  )
}


const PatchProgress = ({ rollout }: { rollout: Rollout }) => {
  const classes = useStyles();
  let versions = rollout.stages.flatMap(stage => stage.updates).filter(u => u.replica_release.name == rollout.latest_release.name).map(u => [u.replica_release, ...u.patches_available]);

  return (
    <>
      {
        versions.length > 0 && <>
          <Typography variant="h6" className={classes.rolloutHeader}>Patch version distribution</Typography>
          {
            _(
              versions
            ).groupBy(
              (v) => v[0].name,
            ).map((versionsGroup, _) => {
              let versions = versionsGroup.sort((a, b) => b.length - a.length)[0];
              return (
                <RolloutProgressStepper versions={versions} />
              )
            }).value()
          }
        </> || <>
          <Typography variant="h6" className={classes.rolloutHeader}>No patches available</Typography>
        </>
      }
    </>
  )
}

function StageIcon({ active, updated }: { active: boolean, updated: boolean }) {
  const classes = useStyles();
  if (active) {
    return <RadioButtonUncheckedIcon />
  } else if (updated) {
    return <RadioButtonCheckedIcon className={classes.inactiveStep} />
  } else {
    return <RadioButtonUncheckedIcon className={classes.inactiveStep} />
  }
}

export default function RolloutsStepper() {
  const classes = useStyles();
  const rollouts = fetchRollouts();

  return (
    <Grid container>
      {rollouts.map(rollout => {
        return (
          <Grid item xs={12}>
            <Paper>
              <Grid
                container
                justifyContent="flex-start"
                alignItems="center"
                alignContent='center'
              >
                <Grid item>
                  <Typography variant="h6" className={classes.rolloutHeader}>
                    Rollout for version <Link className={classes.releaseName} target="_blank" href={`https://github.com/dfinity/ic/commits/${rollout.latest_release.branch}`}>
                      {rollout.latest_release.name}
                    </Link>
                  </Typography>
                </Grid>
                <Grid item>
                  <Chip label={rollout.state} size="small" disabled={rollout.state == "Complete"} style={{ margin: 0 }} />
                </Grid>
              </Grid>
              <Stepper orientation="horizontal" connector={<></>} className={classes.stepper}>
                {_(
                  rollout.stages
                ).groupBy(
                  (s) => {
                    let date = new Date(s.start_timestamp_seconds * 1000);
                    return date.toLocaleDateString("en-US", { weekday: 'long', day: 'numeric', month: 'short', year: undefined });
                  },
                ).map((dayStages, dateString) => {
                  let date = new Date(dateString);
                  let active = date.getDate() == (new Date()).getDate();
                  let activeStep = dayStages.findIndex(s => s.active);
                  return (
                    <Step active={active} key={dateString} expanded style={{ flex: 1 }}>
                      <StepLabel icon={undefined}>{dateString}</StepLabel>
                      <Stepper activeStep={activeStep} orientation="vertical" connector={<></>} className={classes.stepper}>
                        {
                          dayStages.map((stage, i) => {
                            let start = new Date(stage.start_timestamp_seconds * 1000);
                            let stage_label = start.toLocaleTimeString("en-US", { hour: 'numeric', minute: '2-digit' });
                            return (
                              <Step key={stage_label} expanded style={{ flex: 1 }}>
                                <StepLabel icon={<StageIcon active={stage.active} updated={i <= activeStep || date.getDate() < (new Date()).getDate()} />}>{stage_label}</StepLabel>
                                <StepContent>
                                  <RolloutStageContent stage={stage} />
                                </StepContent>
                              </Step>
                            )
                          })
                        }
                      </Stepper>
                    </Step>
                  )
                }).value()}
              </Stepper>
              <Divider />
              <PatchProgress rollout={rollout} />
            </Paper>
          </Grid>
        );
      })}
    </Grid>
  );
}
