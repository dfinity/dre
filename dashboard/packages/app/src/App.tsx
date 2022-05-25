import React from 'react';
import { Navigate, Route } from 'react-router';
import { apiDocsPlugin } from '@backstage/plugin-api-docs';
import {
  catalogPlugin,
} from '@backstage/plugin-catalog';
import { catalogImportPlugin } from '@backstage/plugin-catalog-import';
import {
  scaffolderPlugin,
} from '@backstage/plugin-scaffolder';
import { SearchPage } from '@backstage/plugin-search';
import { UserSettingsPage } from '@backstage/plugin-user-settings';
import { apis } from './apis';
import { Root } from './components/Root';

import { AlertDisplay, OAuthRequestDialog } from '@backstage/core-components';
import { createApp } from '@backstage/app-defaults';
import { FlatRoutes } from '@backstage/core-app-api';
import { TopologyPage } from './components/subnets/TopologyPage';
import { ReleasePage } from './components/subnets/ReleasePage';
import { QueryClient, QueryClientProvider, useQuery } from 'react-query';

import { CssBaseline, ThemeProvider } from '@material-ui/core';
import { BackstageTheme, darkTheme } from '@backstage/theme';
/**
 * The `@backstage/core-components` package exposes this type that
 * contains all Backstage and `material-ui` components that can be
 * overridden along with the classes key those components use.
 */
import { BackstageOverrides } from '@backstage/core-components';
import { QualificationPage } from './components/subnets/QualificationPage';

import { fetchVersion, get_network } from './components/subnets/fetch';

export const createCustomThemeOverrides = (
  theme: BackstageTheme,
): BackstageOverrides => {
  return {
    MuiStepContent: {
      last: {
        borderLeft: `1px solid ${theme.palette.grey[600]}`,
      },
    },
  };
};

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retryDelay: attemptIndex => Math.min(1000 * 2 ** attemptIndex, 30000),
    },
  },
})

const app = createApp({
  apis,
  bindRoutes({ bind }) {
    bind(catalogPlugin.externalRoutes, {
      createComponent: scaffolderPlugin.routes.root,
    });
    bind(apiDocsPlugin.externalRoutes, {
      registerApi: catalogImportPlugin.routes.importPage,
    });
    bind(scaffolderPlugin.externalRoutes, {
      registerComponent: catalogImportPlugin.routes.importPage,
    });
  },
  themes: [{
    id: 'dfinity-theme',
    title: 'DFINITY Theme',
    variant: 'dark',
    Provider: ({ children }) => (
      <ThemeProvider theme={darkTheme}>
        <CssBaseline>{children}</CssBaseline>
      </ThemeProvider>
    ),
  }]
});

const AppProvider = app.getProvider();
const AppRouter = app.getRouter();

const routes = (
  <FlatRoutes>
    <Navigate key="/" to="/network/mercury/topology" />
    {/* <Route path="/catalog" element={<CatalogIndexPage />} />
    <Route
      path="/catalog/:namespace/:kind/:name"
      element={<CatalogEntityPage />}
    >
      {entityPage}
    </Route> */}
    {/* <Route path="/docs" element={<TechdocsPage />} /> */}
    {/* <Route path="/create" element={<ScaffolderPage />} /> */}
    {/* <Route path="/api-docs" element={<ApiExplorerPage />} /> */}
    {/* <Route
      path="/tech-radar"
      element={<TechRadarPage width={1500} height={800} />}
    /> */}
    {/* <Route path="/catalog-import" element={<CatalogImportPage />} /> */}
    <Route path="/search" element={<SearchPage />} />
    <Route path="/settings" element={<UserSettingsPage />} />

    <Route path="/network/mercury/topology" element={<TopologyPage network='Mainnet' />} />
    <Route path="/network/staging/topology" element={<TopologyPage network='Staging' />} />
    <Route path="/release" element={<ReleasePage />} />
    <Route path="/qualification" element={<QualificationPage />} />
  </FlatRoutes>
);

const StateRefresh = ({ children }: { children: React.ReactNode }) => {
  let { data: version } = useQuery<number, Error>("version", async () =>
    await fetchVersion().then((res) => res.json())
    , {
      onSuccess: (data) => {
        if (data !== version) {
          queryClient.invalidateQueries(`${get_network()}_subnets`);
          queryClient.invalidateQueries(`${get_network()}_nodes`);
          queryClient.invalidateQueries(`${get_network()}_operators`);
          queryClient.invalidateQueries(`${get_network()}_guests`);
        }
      },
      refetchInterval: 1000,
      notifyOnChangeProps: ['data'],
    });

  return (<>
    {children}
  </>
  )
}

const App = () => (
  <AppProvider>
    <QueryClientProvider client={queryClient}>
      <StateRefresh>
        <AlertDisplay />
        <OAuthRequestDialog />
        <AppRouter>
          <Root>{routes}</Root>
        </AppRouter>
      </StateRefresh>
    </QueryClientProvider>
  </AppProvider>
);


export default App;
