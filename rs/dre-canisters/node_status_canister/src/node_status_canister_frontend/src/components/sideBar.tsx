import { Container, List, ListItem, ListItemButton, ListItemIcon, ListItemText, SvgIcon } from "@mui/material";
import * as React from "react";
import AppsIcon from '@mui/icons-material/Apps';
import TableViewIcon from '@mui/icons-material/TableView';
import { View } from "..";
import { FC } from "react";

interface SideBarProps {
    setViewOption: (view: View) => void
}

export const SideBar: FC<SideBarProps> = ({ setViewOption }) => <Container>
    <List>
        <ListItem disablePadding onClick={() => setViewOption(View.CARD)}>
            <ListItemButton>
                <ListItemIcon>
                    <AppsIcon />
                </ListItemIcon>
                <ListItemText primary="Card view" />
            </ListItemButton>
        </ListItem>
        <ListItem disablePadding onClick={() => setViewOption(View.TABLE)}>
            <ListItemButton>
                <ListItemIcon>
                    <TableViewIcon />
                </ListItemIcon>
                <ListItemText primary="Table view" />
            </ListItemButton>
        </ListItem>
    </List>
</Container>