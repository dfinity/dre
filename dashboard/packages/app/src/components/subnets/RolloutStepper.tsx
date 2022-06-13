import React from 'react';
import { makeStyles, Theme, createStyles } from '@material-ui/core/styles';
import { Stepper, Step, StepLabel, StepContent, Typography, Chip, Link, Grid, Divider } from '@material-ui/core';
import DoneIcon from '@material-ui/icons/Done';
import CheckCircleOutlineRoundedIcon from '@material-ui/icons/CheckCircleOutlineRounded';
import SyncIcon from '@material-ui/icons/Sync';
import HourglassEmptyIcon from '@material-ui/icons/HourglassEmpty';
import AvTimerIcon from '@material-ui/icons/AvTimer';
import HowToVoteIcon from '@material-ui/icons/HowToVote';
import OpenInNewIcon from '@material-ui/icons/OpenInNew';
import { green, lightBlue, grey, purple, amber, orange } from '@material-ui/core/colors';
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
    progressIcon: {
      color: lightBlue[500],
      animation: "$spin 4s linear infinite",
    },
    pauseIcon: {
      color: grey[500],
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
    completeStep: {
      color: green[500],
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
    case "executed":
      return <SyncIcon className={classes.progressIcon} />
    case "submitted":
      return <HowToVoteIcon className={classes.proposalPendingIcon} />
    case "complete":
      return <DoneIcon className={classes.successIcon} />
    case "baking":
      return <AvTimerIcon className={classes.bakeIcon} />
    case "scheduled":
      return <HourglassEmptyIcon className={classes.pauseIcon} />
  }
}

const RolloutStageContent = ({ stage }: { stage: RolloutStage }) => {
  const classes = useStyles();
  return (
    <Grid container direction='column' spacing={0}>
      {stage.updates.map(update => <Grid item>
        <Chip
          size="small"
          label={`v.${update.replica_release.commit_hash.substring(0, 7)} - ${update.subnet_id.split('-')[0]} - ${update.subnet_name}`}
          onClick={() => window.open(`https://github.com/dfinity/ic/commits/${update.replica_release.commit_hash}`)}
          onDelete={() => window.open(`https://dashboard.internetcomputer.org/proposal/${update.proposal?.info?.id}`)}
          icon={<SubnetUpdateStateIcon state={update.state} />}
          deleteIcon={<OpenInNewIcon />}
          disabled={update.state == "complete"}
          variant={update.state == "scheduled" && "outlined" || "default"}
          className={classes.updateChip}
        />
      </Grid>)}
    </Grid>
  )
}


const PatchProgress = ({ rollout }: { rollout: Rollout }) => {
  const classes = useStyles();
  let versions = rollout.stages.flatMap(stage => stage.updates).filter(u => u.patches_available.length > 0).map(u => [u.replica_release, ...u.patches_available]);
  console.log("Versions", versions);

  return (
    <>
      {
        versions.length > 0 && <>
          <Typography variant="h6" className={classes.rolloutHeader}>Patches available</Typography>
          {
            _(
              versions
            ).groupBy(
              (v) => v[0].name,
            ).map((versionsGroup, _) => {
              let versions = versionsGroup.sort((a, b) => b.length - a.length)[0];
              console.log("group Versions", versions);
              return (
                <RolloutProgressStepper updates={rollout.stages.flatMap(s => s.updates).filter(u => versions.find(p => p.commit_hash === u.replica_release.commit_hash))} versions={versions} />
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

export default function RolloutsStepper() {
  const classes = useStyles();
  // const [activeStep, _] = React.useState(3);
  // const subnets = fetchSubnets();
  const rollouts = fetchRollouts();

  return (
    <>
      {rollouts.map(rollout => {

        return (
          <div className={classes.root}>
            <Typography variant="h6" className={classes.rolloutHeader}>
              Rolling out <Link className={classes.releaseName} target="_blank" href={`https://github.com/dfinity/ic/commits/${rollout.latest_release.branch}`}>
                {rollout.latest_release.name}
              </Link>
            </Typography>
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
                return (
                  <Step active={active} key={dateString} expanded style={{ flex: 1 }}>
                    <StepLabel icon={active && <CheckCircleOutlineRoundedIcon className={classes.completeStep} /> || undefined}>{dateString}</StepLabel>
                    <Stepper activeStep={dayStages.findIndex(s => s.active)} orientation="vertical" connector={<></>} className={classes.stepper}>
                      {
                        dayStages.map(stage => {
                          let start = new Date(stage.start_timestamp_seconds * 1000);
                          let stage_label = start.toLocaleTimeString("en-US", { hour: 'numeric', minute: '2-digit' });
                          return (
                            <Step key={stage_label} expanded style={{ flex: 1 }}>
                              <StepLabel icon={stage.active && <CheckCircleOutlineRoundedIcon className={classes.completeStep} /> || undefined}>{stage_label}</StepLabel>
                              <StepContent>
                                {/* {Array.from(new Set(stage.subnets.filter(s => s.proposal_id).map(s => s.proposal_id!!))).map(id =>
                            <><Link href={`https://dashboard.internetcomputer.org/proposal/${id}`} target="_blank">Proposal {id}</Link><br /></>
                          )} */}
                                <RolloutStageContent stage={stage} />
                                {/* <div className={classes.actionsContainer}>
                            <div>
                              <Button
                                disabled={activeStep === 0}
                                onClick={handleBack}
                                className={classes.button}
                              >
                                Back
                              </Button>
                              <Button
                                variant="contained"
                                color="primary"
                                onClick={handleNext}
                                className={classes.button}
                              >
                                {activeStep === steps.length - 1 ? 'Finish' : 'Next'}
                              </Button>
                            </div>
                          </div> */}
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
          </div>
        );
      })}
    </>
  );
}
