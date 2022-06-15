/*
 * Copyright 2020 The Backstage Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import React, { useContext, PropsWithChildren } from 'react';
import { Link, makeStyles } from '@material-ui/core';
import HomeIcon from '@material-ui/icons/Home';
import LogoFull from './LogoFull';
import LogoIcon from './LogoIcon';
import { NavLink } from 'react-router-dom';
import { Settings as SidebarSettings } from '@backstage/plugin-user-settings';
import { SidebarSearch } from '@backstage/plugin-search';
import SettingsEthernetSharpIcon from '@material-ui/icons/SettingsEthernetSharp';
import LocalShippingIcon from '@material-ui/icons/LocalShipping';
import AssignmentTurnedInIcon from '@material-ui/icons/AssignmentTurnedIn';
import {
  Sidebar,
  SidebarPage,
  sidebarConfig,
  SidebarContext,
  SidebarItem,
  SidebarDivider,
  SidebarSpace,
  SidebarScrollWrapper,
  SidebarSubmenu,
  SidebarSubmenuItem,
} from '@backstage/core-components';

const useSidebarLogoStyles = makeStyles({
  root: {
    width: sidebarConfig.drawerWidthClosed,
    height: 3 * sidebarConfig.logoHeight,
    display: 'flex',
    flexFlow: 'row nowrap',
    alignItems: 'center',
    marginBottom: -14,
  },
  link: {
    width: sidebarConfig.drawerWidthClosed,
    marginLeft: 24,
  },
});

const SidebarLogo = () => {
  const classes = useSidebarLogoStyles();
  const { isOpen } = useContext(SidebarContext);

  return (
    <div className={classes.root}>
      <Link
        component={NavLink}
        to="/"
        underline="none"
        className={classes.link}
      >
        {isOpen ? <LogoFull /> : <LogoIcon />}
      </Link>
    </div>
  );
};

const useSidebarStyles = makeStyles({
  sidebarPage: {
    "& > div": {
      height: "auto",
    }
  },
})

export const Root = ({ children }: PropsWithChildren<{}>) => {
  const classes = useSidebarStyles();
  return (
    <SidebarPage>
      <div className={classes.sidebarPage}>
        <Sidebar>
          <SidebarLogo />
          <SidebarSearch />
          <SidebarDivider />
          {/* Global nav, not org-specific */}
          <SidebarItem icon={HomeIcon} to="/" text="Home" />
          <SidebarItem icon={SettingsEthernetSharpIcon} text="Network">
            <SidebarSubmenu title="Network">
              <SidebarSubmenuItem to="/network/mercury/topology" title="Mainnet" />
              <SidebarSubmenuItem to="/network/staging/topology" title="Staging" />
            </SidebarSubmenu>
          </SidebarItem>
          <SidebarItem icon={LocalShippingIcon} to="/release" text="Release" />
          <SidebarItem icon={AssignmentTurnedInIcon} to="/qualification" text="Qualification" />

          {/* <SidebarItem icon={ExtensionIcon} to="api-docs" text="APIs" /> */}
          {/* <SidebarItem icon={LibraryBooks} to="docs" text="Docs" /> */}
          {/* <SidebarItem icon={CreateComponentIcon} to="create" text="Create..." /> */}
          {/* End global nav */}
          <SidebarDivider />
          <SidebarScrollWrapper>
            {/* <SidebarItem icon={MapIcon} to="tech-radar" text="Tech Radar" /> */}
          </SidebarScrollWrapper>
          <SidebarSpace />
          <SidebarDivider />
          <SidebarSettings />
        </Sidebar>

        {children}

      </div>
    </SidebarPage>
  );
}
