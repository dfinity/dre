import React from 'react';
import { makeStyles, Button, Grid, CardActions, CardContent, Card, CardHeader } from '@material-ui/core';
import { green } from '@material-ui/core/colors';


interface Release {
  branchName: string
  recommended: boolean
  checksumShort: string
}

const releases: Release[] = [
  { branchName: "rc--2021-07-26_18-04", checksumShort: "1d2d88e2", recommended: false },
  { branchName: "rc--2021-07-27_18-03", checksumShort: "b7631cfb", recommended: false },
  { branchName: "rc--2021-07-28_18-03", checksumShort: "9e5d4865", recommended: false },
  { branchName: "rc--2021-07-29_18-03", checksumShort: "0852c549", recommended: false },
  { branchName: "rc--2021-07-30_18-04", checksumShort: "f607aafd", recommended: false },
  { branchName: "rc--2021-07-31_18-03", checksumShort: "f607aafd", recommended: false },
  { branchName: "rc--2021-08-01_18-03", checksumShort: "ce243135", recommended: false },
  { branchName: "rc--2021-08-02_18-03", checksumShort: "c47a773b", recommended: false },
  { branchName: "rc--2021-08-03_18-04", checksumShort: "68eb90c1", recommended: true },
  { branchName: "rc--2021-08-04_18-03", checksumShort: "35956638", recommended: false },
  { branchName: "rc--2021-08-05_18-04", checksumShort: "6bfc26ac", recommended: false },
]

const useStyles = makeStyles(_ => {
  const recommendedColor = green[500];
  return ({
    recommendedBox: {
      borderWidth: "1px",
      borderStyle: "solid",
      borderColor: recommendedColor,
      borderRadius: "5px",
    },
    defaultBox: {
      marginTop: "15px",
      borderWidth: "0",
    },
    recommendedLegend: {
      color: recommendedColor,
    },
    recommendedButton: {
      backgroundColor: recommendedColor,
    },
  })
});


const formatTimeFromBranchName = (branchName: string) => {
  const months = ["January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"];
  var m = months[parseInt(branchName.substr(9, 2)) - 1].substr(0, 3),
    d = branchName.substr(12, 2),
    hour = branchName.substr(15, 2),
    minute = branchName.substr(18, 2);
  return `${m} ${d.replace(/^0/, "")} @${hour}:${minute}`
}

const Release = ({ release }: { release: Release }) => {
  const classes = useStyles();

  return (
    <fieldset className={release.recommended && classes.recommendedBox || classes.defaultBox}>
      {release.recommended && <legend className={classes.recommendedLegend}>Recommended</legend>}
      <Card>
        {/* <CardActionArea> */}
        <CardHeader
          title={`RC ${formatTimeFromBranchName(release.branchName)}`}
          subheader={release.checksumShort}
        />
        <CardContent>
          {/* <Typography gutterBottom variant="h5" component="h2">
              Release {dateFromBranchName(release.branchName)}
            </Typography> */}
          <img src={`https://gitlab.com/dfinity-lab/core/dfinity/badges/${release.branchName}/pipeline.svg`} />
        </CardContent>
        {/* </CardActionArea> */}
        <CardActions>
          <Button size="small" className={release.recommended && classes.recommendedButton || "primary"}>
            Rollout
          </Button>
          {/* <Button size="small" color="primary">
            Rollout
          </Button> */}
        </CardActions>
      </Card>
    </fieldset>
  )
}

export const ReleaseList = () => {
  return (
    <Grid container spacing={0} alignItems="center" justifyContent="space-evenly">
      {releases.slice(releases.length - 7).map(r =>
        <Grid item>
          <Release release={r} />
        </Grid>
      )}
    </Grid>
  )
}
