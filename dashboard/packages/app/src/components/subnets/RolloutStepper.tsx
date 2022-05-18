import React from 'react';
import { makeStyles, Theme, createStyles } from '@material-ui/core/styles';
import { Stepper, Step, StepLabel, StepContent, Typography, ListItem, List, Avatar, ListItemAvatar, ListItemText, Chip, Link, Grid, Divider } from '@material-ui/core';
import DoneIcon from '@material-ui/icons/Done';
import CheckCircleOutlineRoundedIcon from '@material-ui/icons/CheckCircleOutlineRounded';
import SyncIcon from '@material-ui/icons/Sync';
import HourglassEmptyIcon from '@material-ui/icons/HourglassEmpty';
import { green, lightBlue, grey, purple, blue, amber } from '@material-ui/core/colors';
import { Rollout, RolloutStage, SubnetRolloutStatus } from './types';
import { fetchRollout } from './fetch';
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
    }
  }),
);

const SubnetRolloutStateIcon = ({ status }: { status: SubnetRolloutStatus }) => {
  const classes = useStyles();
  if (status.upgrading) {
    return <SyncIcon className={classes.progressIcon} />
  } else if (status.latest_release && !status.upgrading) {
    return <DoneIcon className={classes.successIcon} />
  } else {
    return <HourglassEmptyIcon className={classes.pauseIcon} />
  }
}

const PillChip = ({ label, value, color }: { label: React.ReactNode, value: React.ReactNode, color: string }) => {
  const classes = useStyles();
  return (
    <Chip
      className={classes.versionChip}
      size="small"
      variant="outlined"
      label={
        <Grid container wrap="nowrap">
          <Grid item style={{ background: color }} xs>
            {label}
          </Grid>
          <Divider orientation="vertical" />
          <Grid item xs>
            {value}
          </Grid>
        </Grid>
      }
    />
  )
}

const RolloutStageContent = ({ stage }: { stage: RolloutStage }) => {
  const classes = useStyles();
  return (
    <List>
      {stage.subnets.map(subnet => (
        <>
          <ListItem>
            <ListItemAvatar>
              <Avatar className={subnet.patches_available.length >= 1 && classes.patchAvailableAvatar || ""}>
                <SubnetRolloutStateIcon status={subnet} />
              </Avatar>
            </ListItemAvatar>
            <ListItemText disableTypography>
              {subnet.name}<br />({subnet.principal.split('-')[0]})
            </ListItemText>
          </ListItem>

          <PillChip
            label="Version"
            value={
              <Link
                href={`https://github.com/dfinity/ic/commits/${subnet.replica_release.commit_hash}`}
                target="_blank"
              >
                {subnet.replica_release.commit_hash.substring(0, 7)}
              </Link>
            }
            color={blue[500]}
          />

          {
            subnet.proposal?.pending && <PillChip
              label="Proposal"
              value={
                <Link
                  href={`https://dashboard.internetcomputer.org/proposal/${subnet.proposal?.id}`}
                  target="_blank"
                >
                  {subnet.proposal?.id}
                </Link>
              }
              color={purple[500]}
            />
          }
        </>
      ))
      }
    </List >
    // <Grid container>
    //   {rolloutStages[step].map(s => (
    //     <Grid item>
    //       <Card>
    //         <CardHeader
    //           title={`Subnet ${s.split("-")[0]}`}
    //           subheader={`Version: ${topology.subnets[s].records[0].value.replica_version_id.substr(0,7)}`}
    //         />
    //       </Card>
    //     </Grid>
    //   ))}
    // </Grid>
  )
}

// const useRolloutStepIconStyles = makeStyles({
//   root: {
//     color: '#eaeaf0',
//     display: 'flex',
//     height: 22,
//     alignItems: 'center',
//   },
//   active: {
//     color: green[500],
//   },
//   circle: {
//     width: 8,
//     height: 8,
//     borderRadius: '50%',
//     backgroundColor: 'currentColor',
//   },
//   completed: {
//     color: green[500],
//     zIndex: 1,
//     fontSize: 18,
//   },
// });

// function RolloutStepIcon(props: StepIconProps) {
//   const classes = useRolloutStepIconStyles();
//   const { active, completed } = props;

//   return (
//     <div
//       className={clsx(classes.root, {
//         [classes.active]: active,
//       })}
//     >
//       {completed ? <DoneRoundedIcon className={classes.completed} /> : <div className={classes.circle} />}
//     </div>
//   );
// }

const PatchProgress = ({ rollout }: { rollout: Rollout }) => {
  const classes = useStyles();
  let versions = rollout.stages.flatMap(stage => stage.subnets).filter(s => s.patches_available.length > 0).map(s => [s.replica_release, ...s.patches_available]);
  console.log("Versions", versions);

  return (
    <>
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
            <RolloutProgressStepper subnets={rollout.stages.flatMap(s => s.subnets).filter(s => versions.find(p => p.commit_hash === s.replica_release.commit_hash))} versions={versions} />
          )
        }).value()
      }
    </>
  )
}

export default function RolloutStepper() {
  const classes = useStyles();
  // const [activeStep, _] = React.useState(3);
  // const subnets = fetchSubnets();
  const rollout = fetchRollout();
  const activeStageIndex = rollout.stages.findIndex(stage => stage.subnets.some(s => (s.latest_release && s.upgrading_release) || !s.latest_release));
  const activeStep = activeStageIndex == -1 ? rollout.stages.length : activeStageIndex;

  return (
    <div className={classes.root}>
      <Typography variant="h6" className={classes.rolloutHeader}>
        Rolling out <Link className={classes.releaseName} target="_blank" href={`https://github.com/dfinity/ic/commits/${rollout.release.branch}`}>
          {rollout.release.name}
        </Link>
      </Typography>
      <Stepper activeStep={activeStep} orientation="horizontal" connector={<></>} className={classes.stepper}>
        {rollout.stages.map((stage, i) => (
          <Step key={stage.name} expanded style={{ flex: 1 }}>
            <StepLabel icon={activeStep > i && <CheckCircleOutlineRoundedIcon className={classes.completeStep} /> || undefined}>{stage.name}</StepLabel>
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
        ))}
      </Stepper>
      <Divider />
      <PatchProgress rollout={rollout} />
    </div>
  );
}
