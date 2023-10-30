import React from 'react';
import { Chip, Divider, Grid, LinearProgress, ThemeProvider, Typography, createTheme } from '@material-ui/core';
import Alert from '@material-ui/lab/Alert';

import { Table, Page, Header, Content, HeaderLabel } from '@backstage/core-components';
import { fetchChangePreview } from './fetch';
import { useRouteRefParams } from '@backstage/core-plugin-api';
import { subnetChangePreviewRouteRef } from '../../App';
import { green, lightBlue, purple, red } from '@material-ui/core/colors';
import { Coefficients } from './types';
import { LinearProgressProps } from '@material-ui/core/LinearProgress';
import HowToVoteIcon from '@material-ui/icons/HowToVote';
import Box from '@material-ui/core/Box';

export default {
  title: 'Data Display/Table',
  component: Table,
};

const snakeToSpacedCapitalized = (str: string) =>
  str.toLowerCase().replace(/((\b|[-_])[a-z])/g, (group) =>
    group
      .toUpperCase()
      .replace('-', ' ')
      .replace('_', ' ')
  );

const improvementBarPositiveTheme = createTheme({
  overrides: {
    // Style sheet name ⚛️
    MuiLinearProgress: {
      // Name of the rule
      bar1Buffer: {
        background: green[800],
      },
      bar2Buffer: {
        background: green[500],
      },
      dashed: {
        backgroundImage: "none !important",
        background: "rgba(255, 255, 255, 0.2)",
        animation: "none !important",
      }
    },
  },
});

const improvementBarNegativeTheme = createTheme({
  overrides: {
    // Style sheet name ⚛️
    MuiLinearProgress: {
      // Name of the rule
      bar1Buffer: {
        background: red[500],
      },
      bar2Buffer: {
        background: red[200],
      },
      dashed: improvementBarPositiveTheme?.overrides?.MuiLinearProgress?.dashed,
    },
  },
});

const improvementBarNeutralTheme = createTheme({
  overrides: {
    // Style sheet name ⚛️
    MuiLinearProgress: {
      // Name of the rule
      bar1Determinate: {
        background: lightBlue[200],
      },
      colorPrimary: {
        backgroundColor: "rgba(255, 255, 255, 0.2)",
      }
    },
  },
});

function ImprovementBar(props: LinearProgressProps & { before: number, after: number, max_score: number, label: string }) {
  let before = props.before / props.max_score * 100;
  let after = props.after / props.max_score * 100;
  return (
    <Box display="flex" alignItems="center">
      <Box minWidth={150} alignItems="right" style={{ marginRight: 8 }}>
        <Typography variant="body2" align='right' style={{ fontWeight: "bold" }}>
          {snakeToSpacedCapitalized(props.label)}
        </Typography>
      </Box>
      <Box width="300px" mr={1}>
        {(() => {
          if (before > after) {
            return <ThemeProvider theme={improvementBarNegativeTheme}>
              <LinearProgress variant="buffer" {...props} value={after} valueBuffer={before} />
            </ThemeProvider>
          } else if (after > before) {
            return <ThemeProvider theme={improvementBarPositiveTheme}>
              <LinearProgress variant="buffer" {...props} value={before} valueBuffer={after} />
            </ThemeProvider>
          } else {
            return <ThemeProvider theme={improvementBarNeutralTheme}>
              <LinearProgress variant="determinate" {...props} value={after} />
            </ThemeProvider>
          }
        })()}
      </Box>
      <Box minWidth={35}>
        <Typography variant="body2">
          {+parseFloat(props.after.toString()).toFixed(2)}
          {(() => {
            if (before > after) {
              return <span style={{ color: red[500] }}> ({+parseFloat((after - before).toString()).toFixed(2)})%</span>
            } else if (after > before) {
              return <span style={{ color: green[500] }}> (+{+parseFloat((after - before).toString()).toFixed(2)})%</span>
            } else {
              return ""
            }
          })()}
        </Typography>
      </Box>
    </Box>
  );
}

export const SubnetChangePage = ({ network }: { network: string }) => {
  const params = useRouteRefParams(subnetChangePreviewRouteRef);
  const change = fetchChangePreview(params.subnet);
  console.log(change);

  return (
    <Page themeId="other">
      <Header title="Network Topology">
        <HeaderLabel label="Owner" value="Release Team" />
        <HeaderLabel label="Lifecycle" value={network} />
      </Header>
      <Content>
        <Grid container spacing={4}>
          <Grid item xs={12}>
            <Typography variant='h3'>Subnet <span style={{ fontFamily: "Roboto Mono", background: 'rgba(0, 0, 0, 0.2)', padding: 8 }}>{change?.subnet_id}</span></Typography>
          </Grid>
          <Grid item xs={12}>
            <Chip label="Vote on the proposal" component="a" href={`https://nns.ic0.app/proposal/?proposal=${change?.proposal_id}`} clickable target='_blank' icon={<HowToVoteIcon style={{ color: purple[500] }} />} />
          </Grid>
          <Typography variant="h5" style={{ marginLeft: 16 }}>Decentralization scores</Typography>
          <Grid item xs={12} container direction='column'>

            {Object.entries(change?.score_after?.coefficients ?? {}).map(([key, _]) => {
              let max_score = Math.ceil((change?.score_after?.value_counts.node_provider.map(([_, v]) => v).reduce((sum, c) => sum + c) ?? 1) / 3);
              let before = (change?.score_before?.coefficients[key as keyof Coefficients] ?? 0);
              let after = (change?.score_after?.coefficients[key as keyof Coefficients] ?? 0);
              return <ImprovementBar label={key} before={before} after={after} max_score={max_score} />
            })}
            <ImprovementBar
              label="Overall"
              before={change?.score_before?.avg_linear ?? 0}
              after={change?.score_after?.avg_linear ?? 0}
              max_score={Math.ceil((change?.score_after?.value_counts.node_provider.map(([_, v]) => v).reduce((sum, c) => sum + c) ?? 1) / 3)}
            />
          </Grid>
          {change?.comment && <Grid item>
            <Alert variant="filled" severity="warning">
              { change?.comment.split("\n").filter(l => l).map((line, index) => { if (index == 0) {return line} else { return <li>{line}</li> } }) }
            </Alert>
          </Grid>
          }

          <Grid item xs={12}>
            <Typography variant="h5" style={{ marginLeft: 16 }}>Decentralization counts</Typography>
          </Grid>
          <Grid item container xs={12}>
            {Object.entries(change?.feature_diff ?? {}).map(([f, diff], idx) =>
              <Box display="flex">
                <Grid container>
                  <Grid item xs>
                    <Typography variant='subtitle1'>{snakeToSpacedCapitalized(f)}</Typography>

                    {Object.entries(diff).map(([n, [before, after]]) =>
                      <>
                        <Chip size='small' label={n} style={{ fontFamily: 'Roboto Mono' }} disabled={before == after} variant={before == after ? "outlined" : "default"} />
                        <br />
                      </>
                    )}
                  </Grid>
                  <Grid item>
                    <Typography variant='subtitle1'>Count</Typography>
                    {Object.entries(diff).map(([_, [before, after]]) => {
                      if (before > after) {
                        return <><Chip style={{ fontFamily: 'Roboto Mono' }} variant={before == after ? "outlined" : "default"} size='small' label={<>{after}<span style={{ color: red[700] }}> ({after - before})</span></>} /><br /></>
                      } else if (after > before) {
                        return <><Chip style={{ fontFamily: 'Roboto Mono' }} variant={before == after ? "outlined" : "default"} size='small' label={<>{after}<span style={{ color: green[700] }}> (+{after - before})</span></>} /><br /></>
                      } else {
                        return <><Chip style={{ fontFamily: 'Roboto Mono' }} variant={before == after ? "outlined" : "default"} size='small' disabled label={before} /><br /></>
                      }
                    }
                    )}
                  </Grid>

                  {Object.entries(change?.feature_diff ?? {}).length - 1 > idx &&
                    <Grid item>
                      <Divider orientation='vertical' style={{ marginRight: 16 }} />
                    </Grid>
                    ||
                    <></>
                  }
                </Grid>
              </Box>
            )}
          </Grid>
        </Grid>
      </Content>
    </Page>
  )
}
