import React from 'react';
import { makeStyles, Theme, createStyles, withStyles } from '@material-ui/core/styles';
import Stepper from '@material-ui/core/Stepper';
import Step from '@material-ui/core/Step';
import StepLabel from '@material-ui/core/StepLabel';
import StepConnector from '@material-ui/core/StepConnector';
import { ReplicaRelease } from './types';
import { amber } from '@material-ui/core/colors';
import { Chip, Grid, Link, StepContent } from '@material-ui/core';
import RadioButtonCheckedIcon from '@material-ui/icons/RadioButtonChecked'
import { fetchSubnets } from './fetch';

const QontoConnector = withStyles({
  alternativeLabel: {
    top: 10,
    left: 'calc(-50% + 16px)',
    right: 'calc(50% + 16px)',
  },
  active: {
    '& $line': {
      borderColor: amber[500],
    },
  },
  completed: {
    '& $line': {
      borderColor: amber[500],
      '&::after': {
        // https://stackoverflow.com/a/66233926
        content: '""',
        width: 0,
        height: 0,
        borderTop: "5px solid transparent",
        borderBottom: "5px solid transparent",
        borderLeft: `8px solid ${amber[500]}`,
        position: "absolute",
        right: -3,
        top: -4,
      }
    },
  },
  line: {
    borderColor: '#eaeaf0',
    borderTopWidth: 3,
    borderRadius: 1,
  },
})(StepConnector);

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    root: {
      width: '100%',
    },
    button: {
      marginRight: theme.spacing(1),
    },
    instructions: {
      marginTop: theme.spacing(1),
      marginBottom: theme.spacing(1),
    },
    patchStepIcon: {
      color: amber[500],
    },
    chip: {
      fontFamily: 'Roboto Mono',
    },
  }),
);

export default function RolloutProgressStepper({ versions }: { versions: ReplicaRelease[] }) {
  const classes = useStyles();
  const subnets = fetchSubnets();

  return (
    <div className={classes.root}>
      <Stepper alternativeLabel activeStep={versions.length} connector={<QontoConnector />}>
        {versions.map((p) => (
          <Step key={p.commit_hash.substring(0, 7)} expanded>
            <StepLabel icon={<RadioButtonCheckedIcon className={classes.patchStepIcon} />}>
              <Link
                target="_blank"
                color="textPrimary"
                href={`https://github.com/dfinity/ic/commits/${p.commit_hash}`}
              >
                {p.commit_hash.substring(0, 7)}
              </Link>
            </StepLabel>
            <StepContent style={{ border: "none" }}>
              <Grid
                container
                direction="row"
                justifyContent="center"
                alignItems="center"
                alignContent="center"
                spacing={0}
              >
                {
                  Object.values(subnets).sort((a, b) => a.principal.localeCompare(b.principal))
                    .filter(s => s.replica_version === p.commit_hash)
                    .map(s =>
                      <Grid item spacing={0}>
                        <Chip
                          size="small"
                          variant='outlined'
                          className={classes.chip}
                          label={`${s.principal.split("-")[0]}`}
                        />
                      </Grid>
                    )
                }
              </Grid>
            </StepContent>
          </Step>
        ))}
      </Stepper>
    </div>
  );
}
