import React from 'react';
import { FormControlLabel, FormGroup, Grid, Switch } from '@material-ui/core';

import { Table, Page, Header, Content, HeaderLabel, Gauge, InfoCard } from '@backstage/core-components';
import { AvailableNodes } from './AvailableNodes';
import SubnetsBoard from './SubnetsBoard';
import Decentralization from './Decentralization';
import { Guest, Node } from './types';
import SubnetsMatrix from './SubnetsMatrix';
import NodeSearch from './NodeSearch';
import { fetchGuests, fetchNodes } from './fetch';

export default {
  title: 'Data Display/Table',
  component: Table,
};

const datacenterProgress = (guests: Guest[], nodes: Node[], dc: string) => {
  let datacenterGuests = guests.filter(h => h.datacenter == dc)
  return {
    onboarded: nodes.filter(n => datacenterGuests.find(g => g.name == n.hostname)).length,
    total: datacenterGuests.length
  }
}

const datacenters = (guests: Guest[], nodes: Node[]) => Array.from(guests.reduce((r, c) => r.add(c.datacenter), new Set<string>())).map(dc => ({
  name: dc,
  operator: nodes.find(
    n => n.operator?.datacenter?.name == dc
  )?.operator?.datacenter?.owner?.name ?? "Unknown"
}));

export const TopologyPage = ({network}: {network: string}) => {
  const guests = fetchGuests();

  const nodes = fetchNodes();

  const [showMatrix, setShowMatrix] = React.useState(false);

  const handleShowMatrix = (event: React.ChangeEvent<HTMLInputElement>) => {
    setShowMatrix(event.target.checked);
  };

  const [searchNodes, setSearchNodes] = React.useState(false);

  const matrixSwitch = (
    <FormGroup row>
      <FormControlLabel
        control={<Switch checked={showMatrix} onChange={handleShowMatrix} name="matrix" />}
        label="Matrix view"
      />
    </FormGroup>
  );

  return (
    <Page themeId="other">
      <Header title="Network Topology">
        <HeaderLabel label="Owner" value="Release Team" />
        <HeaderLabel label="Lifecycle" value={network} />
      </Header>
      <Content>
        <Grid container alignItems="center">
          <Grid item xs={12}>
            <NodeSearch expand={searchNodes} onSearchChange={(searchText: string) => setSearchNodes(searchText != "")} />
          </Grid>
          <Grid item>
            {matrixSwitch}
          </Grid>
          <Grid item xs={12}>
            <div style={{ display: showMatrix ? 'none' : 'inherit' }}>
              <SubnetsBoard />
            </div>
            <div style={{ display: !showMatrix ? 'none' : 'inherit' }}>
              <SubnetsMatrix />
            </div>
          </Grid>
          <Grid item xs={12}>
            <AvailableNodes />
          </Grid>
          <Grid item xs={12}>
            <Decentralization />
          </Grid>
          <Grid item xs={12}>
            <Grid container alignItems="stretch">
              {datacenters(guests ?? [], Object.values(nodes ?? {})).map(dc => ({ ...dc, progress: datacenterProgress(guests ?? [], Object.values(nodes ?? {}), dc.name) || 0 })).sort((dc1, dc2) => dc1.progress.onboarded / dc1.progress.total > dc2.progress.onboarded / dc2.progress.total ? -1 : 1).map(dc => {
                return (
                  <Grid item>
                    <InfoCard title={dc.name} subheader={<>Operator: {dc.operator}<br />Onboarded: {dc.progress.onboarded} / {dc.progress.total}</>}>
                      <Gauge value={dc.progress.onboarded / dc.progress.total}></Gauge>
                    </InfoCard>
                  </Grid>
                )
              })}
            </Grid>
          </Grid>
        </Grid>
      </Content>
    </Page>
  )
}

// export const subnetsPage = <subnetsTable />;
