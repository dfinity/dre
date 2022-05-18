import React from 'react';
import { makeStyles, Theme, createStyles } from '@material-ui/core/styles';
import { Button, Checkbox, Divider, FormControlLabel, FormGroup, Grid, IconButton, List, ListItem, ListItemAvatar, ListItemSecondaryAction, ListItemText, Paper } from '@material-ui/core';
import Typography from '@material-ui/core/Typography';
import AccountCircleIcon from '@material-ui/icons/AccountCircle';
import { green, grey } from '@material-ui/core/colors';
import AddCircle from '@material-ui/icons/AddCircle';
import RemoveCircle from '@material-ui/icons/RemoveCircle';
import CheckCircle from '@material-ui/icons/CheckCircle';

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    root: {
      width: '100%',
    },
    backButton: {
      marginRight: theme.spacing(1),
    },
    instructions: {
      marginTop: theme.spacing(1),
      marginBottom: theme.spacing(1),
    },
    stepIconDone: {
      color: green[500],
    },
    stepIconAddCommit: {
      color: grey[500],
    },
    buttonDone: {
      color: green[500],
    },
    divider: {
      margin: theme.spacing(2),
    },
    flexCenter: {
      display: "flex",
      justifyContent: "center",
    },
  }),
);


interface Commit {
  checksum: string
  message: string
  author: string
  when: string
  picked: boolean
}

const commits: Commit[] = [
  { checksum: "da8587b2ee209df0ce03a20b2226a1d3db6292d9", message: "refactor: fixup workload_generator params (#12694)", author: "Daniel-Bloom-dfinity", when: "committed 3 days ago", picked: false },
  { checksum: "68eb90c19f9d76ed306ecbc836ecaa595023994d", message: "NNS1-541: Add functionality to delete subnets from registry (#12089) …", author: "nzoghb and dfinity-bot", when: "committed 3 days ago", picked: false },
  { checksum: "618d97e6c0b47b8e42ec8061e4604ba8630f70f8", message: "chore: fix grammar in test scripts (#12693)", author: "Daniel-Bloom-dfinity", when: "committed 3 days ago", picked: false },
  { checksum: "48ad7620de57fdbf93b7c77d8fb4606241a0f77b", message: "rs/check.nix: disable the ledger-canister test (#12692) …", author: "basvandijk and ali-dfinity", when: "committed 3 days ago", picked: false },
  { checksum: "43a7757c0da0e4d6e4366049c8522852169b2bf0", message: "Add backup purging logs (#12691) …", author: "chmllr", when: "committed 3 days ago", picked: false },
  { checksum: "92b40c3a31348fe1f5d608116c85a5e759b3d95e", message: "gitlab-ci/docker: upgrade dfx to the latest 0.8.0 (#12689) …", author: "basvandijk and IDX GitLab Automation", when: "committed 3 days ago", picked: false },
  { checksum: "6cd5100cbe89f6a908e98f92fd668a1b84b00dab", message: "[BQ-75] mainnet-ops: print replica version in each subnet (#12686) …", author: "sasa-tomic", when: "committed 3 days ago", picked: true },
  { checksum: "6884afb4f00ee78d42190e1ccfe5165f2a62ea2f", message: "[BQ-74] Always download blessed binaries in upgrades (#12684) …", author: "sasa-tomic", when: "committed 3 days ago", picked: false },
  { checksum: "beaa3e4657eb49f4ad4495a97f4d16ba5f22d22b", message: "Do not keep empty config groups. (#12690) …", author: "egeyar", when: "committed 3 days ago", picked: false },
  { checksum: "caf9478302ae9cf2b197b7aa6487ff73636b1704", message: "prod/tests: fix the disaster_recovery_basic_test (#12685) …", author: "basvandijk", when: "committed 3 days ago", picked: true },
  { checksum: "1a0d76253f3e24d29003a6f84a06a9eaa8301be7", message: "purge states based on cert height from finalized tip (#12290) …", author: "manudrijvers", when: "committed 3 days ago", picked: false },
  { checksum: "5cf92716babffd31bf0bf258268a7f7aeeca0c84", message: "prod/tests: fix ledger_query_update test (#12681) …", author: "basvandijk", when: "committed 3 days ago", picked: false },
  { checksum: "ea76bf82569441e6ca7813b6f6523d6b563a9537", message: "Ignore system time error (#12680) …", author: "chmllr", when: "committed 3 days ago", picked: false },
  { checksum: "d274d2fc83fe4ae3fb90ff1b29b3bd07e27495f9", message: "EXC-345: Add query execution metrics (#12645) …", author: "ulan", when: "committed 3 days ago", picked: false },
  { checksum: "871ffaf3daa48977622416348f6b4fbe867e2ee6", message: "Generate oncall schedule with a single Triage primary and secondary p… …", author: "janeshchhabra and pd-dfinity", when: "committed 3 days ago", picked: false },
  { checksum: "bb24037ceb5b020560955547179e6504bf8ed818", message: "Fix disaster recovery basic test for single host subnet (#12677) …", author: "ninegua", when: "committed 4 days ago", picked: false },
]

const CommitListItem = ({ commit, children }: { commit: Commit, children: React.ReactNode }) => (
  <ListItem>
    <ListItemAvatar>
      <AccountCircleIcon />
    </ListItemAvatar>
    <ListItemText primary={commit.message} secondary={`Authored by @${commit.author} ${commit.when}`} />
    {children}
  </ListItem>
)

const ReleaseHotfix = ({ commits }: { commits: Commit[] }) => {
  const classes = useStyles();

  const [state, setState] = React.useState({
    checkedHotfixAll: true,
    checkedRollout: true,
  });

  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setState({ ...state, [event.target.name]: event.target.checked });
  };
  return (
    <Grid container justifyContent="center">
      <Grid item xs={12} className={classes.flexCenter}>
        <Button variant="contained" size="large" disabled={commits.length === 0} color="primary">Release</Button>
      </Grid>
      <Grid item>
        <FormGroup>
          <FormControlLabel
            className={classes.flexCenter}
            control={<Checkbox checked={state.checkedHotfixAll} onChange={handleChange} name="checkedHotfixAll" />}
            label="Hotfix all production releases"
          />
          <FormControlLabel
            className={classes.flexCenter}
            control={<Checkbox checked={state.checkedRollout} onChange={handleChange} name="checkedRollout" />}
            label="Rollout"
          />
        </FormGroup>
      </Grid>
    </Grid>
  )
}

export default function HotfixReleases() {
  const classes = useStyles();

  // const cherryPickHandlers = commits.map(c => React.useState(c.picked));
  const [cherryPicked, setCherryPicked] = React.useState(new Array<Commit>());

  return (
    <Paper>
      <Grid container>
        <Grid item xs>
          <Typography variant="h4" align="center">@master</Typography>
          <List>
            {commits.map(commit => (
              <CommitListItem commit={commit}>
                <ListItemSecondaryAction>
                  <IconButton edge="end" aria-label="add" disabled={cherryPicked.find(cp => cp.checksum === commit.checksum) !== undefined || commit.picked} onClick={() => setCherryPicked([commit, ...cherryPicked])}>
                    <AddCircle />
                  </IconButton>
                </ListItemSecondaryAction>
              </CommitListItem>
            ))}
          </List>
        </Grid>
        <Divider orientation="vertical" flexItem />
        <Grid item xs>
          <Typography variant="h4" align="center">@release-08-08-2021.hotfix.1&#8608;2</Typography>
          <List>
            {commits.filter(c => c.picked).map(commit => (
              <CommitListItem commit={commit} >
                <ListItemSecondaryAction>
                  <IconButton edge="end" disabled>
                    <CheckCircle className={classes.buttonDone} />
                  </IconButton>
                </ListItemSecondaryAction>
              </CommitListItem>
            ))}
            {cherryPicked.map(commit => (
              <CommitListItem commit={commit} >
                <ListItemSecondaryAction>
                  <IconButton edge="end" aria-label="remove" onClick={() => setCherryPicked(cherryPicked.filter(cp => cp.checksum !== commit.checksum))}>
                    <RemoveCircle />
                  </IconButton>
                </ListItemSecondaryAction>
              </CommitListItem>
            ))}
          </List>
          <Divider className={classes.divider} />
          <ReleaseHotfix commits={cherryPicked} />
        </Grid>
        {/* <Divider orientation="vertical" flexItem /> */}
        {/* <Grid item>
        </Grid> */}
      </Grid>
    </Paper>
  );
}
