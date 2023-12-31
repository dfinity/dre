import React from 'react';
import { Grid } from '@material-ui/core';

import { Page, Header, Content, HeaderLabel } from '@backstage/core-components';
// import { ReleaseList } from './ReleaseList';
import SubnetVersionSearch from './SubnetVersionSearch';
// import HotfixReleases from './HotfixReleases';

export const ReleasePage = () => {
  return (
    <Page themeId="other">
      <Header title="Mainnet Release">
        <HeaderLabel label="Owner" value="Release Team" />
        <HeaderLabel label="Lifecycle" value="Production" />
      </Header>
      <Content>
        <Grid container>
          <Grid item xs={12}>
            <SubnetVersionSearch />
            {/* <RolloutsStepper /> */}
          </Grid>
        </Grid>
      </Content>
    </Page>
  )
}
