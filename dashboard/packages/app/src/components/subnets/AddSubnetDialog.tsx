import React from 'react';
import Button from '@material-ui/core/Button';
import Dialog from '@material-ui/core/Dialog';
import DialogActions from '@material-ui/core/DialogActions';
import DialogContent from '@material-ui/core/DialogContent';
import DialogContentText from '@material-ui/core/DialogContentText';
import DialogTitle from '@material-ui/core/DialogTitle';
import { CodeSnippet } from '@backstage/core-components';
import { Chip, Divider, Grid, List, ListItem, ListItemText, Typography } from '@material-ui/core';
import { Node } from './types';
import { useQuery } from 'react-query';
import { useApi, configApiRef } from '@backstage/core-plugin-api';

const subnetSize = 13;


const generateSubnet = () => {
  const config = useApi(configApiRef);
  const { data } = useQuery<Node[], Error>("create_subnet", () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/create_subnet?size=${subnetSize}`
    ).then((res) => res.json())
  );
  return data ?? [];
}

const generateCreateSubnetInstructions = (nodes: Node[]) => {
  return `
./mainnet-op propose-to-create-subnet SUBNET_NUMBER \\
    ${nodes.map(n => n.principal).join(" \\\n    ")}
`;
}

export default function AddSubnetDialog() {
  const [open, setOpen] = React.useState(false);
  const subnetNodes = generateSubnet();

  const handleClickOpen = () => {
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
  };

  return (
    <React.Fragment>
      <Button color="primary" onClick={handleClickOpen}>
        Add subnet
      </Button>
      <Dialog
        open={open}
        onClose={handleClose}
        aria-labelledby="max-width-dialog-title"
        // fullWidth={true}
        maxWidth="xl"
      >
        <DialogTitle id="max-width-dialog-title">New subnet</DialogTitle>
        <DialogContent>
          <Grid container>
            {subnetNodes.map(n =>
              <Grid item>
                <Chip size="small" label={n.hostname}></Chip>
              </Grid>
            )}
          </Grid>
          <DialogContentText>
          </DialogContentText>
          <Grid container>
            <Grid item xs>
              <Typography variant="subtitle2">Provider count: {new Set(subnetNodes.map(n => n.operator.provider.principal)).size}</Typography>
              <List>
                {subnetNodes.map(n => n.operator.provider.principal).reduce((r: { name: string, count: number }[], c) => {
                  let entry = r.find(p => p.name == c);
                  if (entry === undefined) {
                    r.push({ name: c, count: 0 })
                  }
                  r.find(p => p.name == c)!.count++;
                  return r
                }, []).map(p =>
                  <ListItem>
                    <ListItemText primary={`Provider ${p.name}: ${p.count}`}></ListItemText>
                  </ListItem>
                )}
              </List>
            </Grid>
            <Divider orientation="vertical" />
            <Grid item xs>
              <Typography variant="subtitle2">Operator count: {new Set(subnetNodes.map(n => n.operator.principal)).size}</Typography>
              <List>
                {subnetNodes.map(n => n.operator.datacenter?.owner?.name).reduce((r: { name: string, count: number }[], c) => {
                  let entry = r.find(p => p.name == c);
                  if (entry === undefined) {
                    r.push({ name: c || "Unknown", count: 0 })
                  }
                  r.find(p => p.name == c)!.count++;
                  return r
                }, []).map(p =>
                  <ListItem>
                    <ListItemText primary={`Operator ${p.name}: ${p.count}`}></ListItemText>
                  </ListItem>
                )}
              </List>
            </Grid>
          </Grid>
          <CodeSnippet text={generateCreateSubnetInstructions(subnetNodes)} language="shell" showCopyCodeButton />
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose} color="primary">
            Close
          </Button>
        </DialogActions>
      </Dialog>
    </React.Fragment>
  );
}
