import React from 'react';
import { FormControlLabel, FormGroup, Grid, Switch } from '@material-ui/core';

import { Table, Page, Header, Content, HeaderLabel } from '@backstage/core-components';
import { AvailableNodes } from './AvailableNodes';
import SubnetsBoard from './SubnetsBoard';
import Decentralization from './Decentralization';
import SubnetsMatrix from './SubnetsMatrix';
import NodeSearch from './NodeSearch';

export default {
  title: 'Data Display/Table',
  component: Table,
};

export const TopologyPage = ({ network }: { network: string }) => {
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
        </Grid>
      </Content>
    </Page>
  )
}

// export const subnetsPage = <subnetsTable />;
