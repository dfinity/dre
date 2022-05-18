import React from 'react';
import { Grid } from '@material-ui/core';

import { Page, Header, Content, HeaderLabel } from '@backstage/core-components';

export const QualificationPage = () => {
    return (
        <Page themeId="other">
            <Header title="RC Qualification">
                <HeaderLabel label="Owner" value="Release Team" />
                <HeaderLabel label="Lifecycle" value="Production" />
            </Header>
            <Content>
                <Grid container>
                    <Grid item xs={12}>
                        <iframe
                            src="https://grafana.dfinity.systems/d-solo/NjNXd6Jnz/release-candidate-qualification-and-rollout?orgId=1&refresh=30s&from=now-65d&to=now&panelId=10"
                            style={{ width: "100%", height: 300, border: "none", overflow: "hidden" }}
                        />
                        <iframe src="https://grafana.dfinity.systems/d-solo/NjNXd6Jnz/release-candidate-qualification-and-rollout?orgId=1&refresh=3
            0s&from=now-65d&to=now&panelId=16" style={{ width: "100%", height: "100vh", border: 0 }}></iframe>
                    </Grid>
                </Grid>
            </Content>
        </Page>
    )
}
