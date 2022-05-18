import React from 'react';
import { makeStyles } from '@material-ui/core/styles';
import Table from '@material-ui/core/Table';
import TableBody from '@material-ui/core/TableBody';
import TableCell from '@material-ui/core/TableCell';
import TableContainer from '@material-ui/core/TableContainer';
import TableHead from '@material-ui/core/TableHead';
import TableRow from '@material-ui/core/TableRow';
import Paper from '@material-ui/core/Paper';
import { fetchSubnets } from './fetch';
import { Subnet } from './types';
import { colors, Typography } from '@material-ui/core';

const useStyles = makeStyles({
  table: {
    width: "100%",
  },
});

const SubnetRow = ({ subnet, dcs }: { subnet: Subnet, dcs: string[] }) => {
  return (
    <TableRow key={subnet.principal}>
      <TableCell component="th" scope="row">
        {subnet.metadata.name} ({subnet.principal.split("-")[0]})
      </TableCell>
      {dcs.map(dc => {
        const nodes = subnet.nodes.filter(n => n.operator.datacenter.name === dc);
        return (
          <TableCell style={{ ...{ padding: 2 }, ...(nodes.some(n => n.labels.find(l => l.name == "DFINITY")) ? { background: colors.red[200] } : {}) }}>
            {
              nodes.map(n =>
                <>
                  <Typography variant="body2">
                    {n.hostname}
                  </Typography>
                  <br />
                </>
              )
            }
          </TableCell>
        )
      }
      )}
    </TableRow >
  )
}

export default function SubnetsMatrix() {
  const classes = useStyles();
  const subnets = Object.values(fetchSubnets()).sort((s1, s2) => {
    if (s1.metadata.name === "NNS") {
      return -1
    } else if (s2.metadata.name === "NNS") {
      return 1
    }
    return parseInt(s1.metadata.name.split(" ")[1]) - parseInt(s2.metadata.name.split(" ")[1])
  });
  const dcs = Array.from(subnets.reduce((r, s) => new Set([...r, ...s.nodes.map(n => n.operator.datacenter.name)]), new Set<string>())).sort((d1, d2) => d1.localeCompare(d2));

  return (
    <TableContainer component={Paper}>
      <Table className={classes.table} size="small" aria-label="a dense table">
        <TableHead>
          <TableRow>
            {["Subnet"].concat(dcs).map(h => <TableCell>{h}</TableCell>)}
          </TableRow>
        </TableHead>
        <TableBody>
          {subnets.map((subnet) => <SubnetRow subnet={subnet} dcs={dcs} />)}
        </TableBody>
      </Table>
    </TableContainer>
  );
}
