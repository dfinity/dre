import * as React from 'react';
import AppBar from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import Typography from '@mui/material/Typography';
import Container from '@mui/material/Container';
import { DfinitySvg } from '../utils/svgs';


function NavBar() {
    return (
        <AppBar position="static" sx={{
            marginBottom: 2,
        }}>
            <Container maxWidth="xl">
                <Toolbar disableGutters>
                    {DfinitySvg}
                    <Typography
                        variant="h6"
                        noWrap
                        component="a"
                        href="/"
                        sx={{
                            mr: 2,
                            display: { xs: 'none', md: 'flex' },
                            fontFamily: 'monospace',
                            fontWeight: 700,
                            letterSpacing: '.3rem',
                            color: 'inherit',
                            textDecoration: 'none',
                        }}
                    >
                        DFINITY
                    </Typography>
                </Toolbar>
            </Container>
        </AppBar>
    );
}
export default NavBar;
