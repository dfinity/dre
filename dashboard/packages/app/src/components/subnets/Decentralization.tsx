import React from 'react';
import { CardContent, Grid, Card } from '@material-ui/core';
import { Chart } from "react-google-charts";
import { green, deepOrange, amber } from '@material-ui/core/colors';
import _ from "lodash";
import { fetchNodes } from './fetch';

const commonChartOptions = {
  tooltip: {
    trigger: 'none',
  },
  backgroundColor: "transparent",
  chartArea: {
  },
  hAxis: {
    textStyle: {
      color: 'white',
    },
    gridlines: {
      color: 'transparent',
    }
  },
  vAxis: {
    textStyle: {
      color: 'white',
    },
    gridlines: {
      color: 'transparent',
    }
  },
  titleTextStyle: {
    color: 'white',
  }
}

// const SubnetDecentralizationChart = withTheme(({ theme }: { theme: Theme }) => {
//   return (
//     <Card style={{ height: '100%' }}>
//       <CardContent>
//         <Chart
//           width={'500px'}
//           height={'300px'}
//           chartType="Histogram"
//           loader={<div>Loading Chart</div>}
//           data={[
//             ['Now', 'After'],
//             [.7, .18],
//             [.30, .48],
//             [.82, .83],
//             [.4, .5],
//             [.35, .43],
//             [.27, .36],
//             [.37, .50],
//             [.71, .81],
//             [.38, .43],
//             [.42, .45],
//             [.27, .29],
//             [.19, .35],
//             [.16, .22],
//             [.51, .69],
//             [.66, .84],
//             [.12, .14],
//             [.33, .52],
//             [.29, .37],
//             [.96, .97],
//             [.59, .71],
//           ]}
//           options={_.merge({
//             textStyle: {
//               color: 'white',
//             },
//             title: 'Subnets decentralization ratings',
//             colors: [blue[500], green[500]],
//             interpolateNulls: false,
//             hAxis: {
//               format: 'percent',
//             },

//             bar: {
//               gap: theme.spacing(1),
//             },
//             legend: {
//               position: 'top',
//               maxLines: 2,
//               textStyle: {
//                 color: 'white'
//               },
//             },
//             histogram: {
//               bucketSize: 0.01,
//               maxNumBuckets: 4,
//             }
//           }, commonChartOptions)}
//           rootProps={{ 'data-testid': '5' }}
//         />
//       </CardContent>
//       <CardActions>
//         <Button variant="contained">Optimize</Button>
//       </CardActions>
//     </Card>
//   )
// })

function DistributionChart<Type>({
  title,
  values,
  selector,
  tresholdWarning,
  tresholdCritical
}: {
  title: string,
  values: Type[],
  selector: (value: Type) => string,
  tresholdWarning?: number,
  tresholdCritical?: number,
}) {
  const chartData = Array.from(values.map(selector).reduce((r, a) => r.set(a, 1 + (r.get(a) ?? 0)), new Map<string, number>()).entries());

  return (
    <Card style={{ height: '100%' }}>
      <CardContent>
        <Chart
          chartType="ColumnChart"
          loader={<div>Loading Chart</div>}
          data={[["DC Owner", "Node percentage", { role: "style" }], ...chartData.sort(([, ac], [, bc]) => bc - ac).slice(0, 5).map(e => {
            let ratio = e[1] / chartData.reduce((r, [_, a]) => r + a, 0);
            return [e[0], ratio, (() => {
              if (ratio > (tresholdCritical ?? 1. / 3)) {
                return deepOrange[400];
              } else if (ratio > (tresholdWarning ?? 1. / 5)) {
                return amber[400];
              } else {
                return green[400];
              }
            })()]
          })]}
          options={_.merge({
            title: title,
            vAxis: {
              format: 'percent',
            },
            bar: { groupWidth: '80%' },
            legend: { position: 'none' },
          }, commonChartOptions)}
          rootProps={{ 'data-testid': '6' }}
        />
      </CardContent>
    </Card>
  )
}

export default function Decentralization() {
  let nodes = Object.values(fetchNodes());
  return (
    <Grid container alignItems="stretch">
      <Grid item xs={2}>
        <DistributionChart title="Top datacenter owners" values={nodes} selector={n => n.operator.datacenter.owner.name} />
      </Grid>
      <Grid item xs={2}>
        <DistributionChart title="Top node providers" values={nodes} selector={n => n.operator.provider.name ?? n.operator.provider.principal} />
      </Grid>
      <Grid item xs={2}>
        <DistributionChart title="Top continents" values={nodes} selector={n => n.operator.datacenter.continent} tresholdWarning={1 / 3.} tresholdCritical={1 / 2.} />
      </Grid>
      <Grid item xs={2}>
        <DistributionChart title="Top countries" values={nodes} selector={n => n.operator.datacenter.country} />
      </Grid>
      {/* <Divider orientation="vertical" flexItem /> */}
      {/* <Grid item xs={2}>
        <SubnetDecentralizationChart />
      </Grid> */}
    </Grid>
  );
}
