import React from 'react';
import { makeStyles, Theme, createStyles } from '@material-ui/core/styles';
import DoneIcon from '@material-ui/icons/Done';
import SyncIcon from '@material-ui/icons/Sync';
import HourglassEmptyIcon from '@material-ui/icons/HourglassEmpty';
import AvTimerIcon from '@material-ui/icons/AvTimer';
import HowToVoteIcon from '@material-ui/icons/HowToVote';
import HelpIcon from '@material-ui/icons/Help';
import UpdateIcon from '@material-ui/icons/Update';
import { green, lightBlue, grey, purple, amber, orange, blue, red } from '@material-ui/core/colors';
import { SubnetUpdateState } from './types';
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
        sectionHeader: {
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
        },
        versionDistribution: {
            width: "100%",
        },
        versionChip: {
            fontFamily: 'Roboto Mono',
        }
    }),
);

export const SubnetUpdateStateIcon = ({ state }: { state: SubnetUpdateState }) => {
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

export const stateDescription = (state: SubnetUpdateState) => {
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
