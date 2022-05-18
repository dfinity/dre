import React from 'react';
import { FormControlLabel, FormGroup, Grid, Switch } from '@material-ui/core';

import { Table, Page, Header, Content, HeaderLabel, Gauge, InfoCard } from '@backstage/core-components';
import { AvailableNodes } from './AvailableNodes';
import SubnetsBoard from './SubnetsBoard';
import Decentralization from './Decentralization';
import { useQuery } from 'react-query';
import { Host, Node } from './types';
import { useApi, configApiRef } from '@backstage/core-plugin-api';
import SubnetsMatrix from './SubnetsMatrix';
import NodeSearch from './NodeSearch';

export default {
  title: 'Data Display/Table',
  component: Table,
};

const datacenterProgrees = (hosts: Host[], nodes: Node[], dc: string) => {
  let datacenterHosts = hosts.filter(h => h.datacenter == dc)
  return {
    onboarded: nodes.filter(n => datacenterHosts.find(h => h.name == n.hostname)).length,
    total: datacenterHosts.length
  }
}

const datacenters = (hosts: Host[], nodes: Node[]) => Array.from(hosts.reduce((r, c) => r.add(c.datacenter), new Set<string>())).map(dc => ({
  name: dc,
  operator: nodes.find(
    n => n.operator?.datacenter?.name == dc
  )?.operator?.datacenter?.owner?.name ?? "Unknown"
}));

// const datacenters = (hosts: Host[], nodes: Node[]) => Array.from(hosts.map(h => ({
//   name: h.datacenter,
//   operator: nodes.find(
//     n => n.hostname == h.name
//   )?.operator?.datacenter?.owner?.name ?? "Unknown"
// })).reduce((r, c) => r.add(c), new Set<{ name: string, operator: string }>()));

export const TopologyPage = () => {
  const config = useApi(configApiRef);

  const { data: hosts } = useQuery<Host[], Error>("hosts", () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/hosts`
    ).then((res) => res.json())
  );

  const { data: nodes } = useQuery<{ [principal: string]: Node }, Error>("nodes", () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/nodes`
    ).then((res) => res.json())
  );

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
      <Header title="Mainnet Topology">
        <HeaderLabel label="Owner" value="Release Team" />
        <HeaderLabel label="Lifecycle" value="Production" />
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
              {datacenters(hosts ?? [], Object.values(nodes ?? {})).map(dc => ({ ...dc, progress: datacenterProgrees(hosts ?? [], Object.values(nodes ?? {}), dc.name) || 0 })).sort((dc1, dc2) => dc1.progress.onboarded / dc1.progress.total > dc2.progress.onboarded / dc2.progress.total ? -1 : 1).map(dc => {
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
