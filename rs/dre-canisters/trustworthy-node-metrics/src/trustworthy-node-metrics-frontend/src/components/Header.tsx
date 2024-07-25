import { AppBar, Toolbar, Typography, IconButton, styled } from '@mui/material';

const Title = styled(Typography)(({ theme }) => ({
    flexGrow: 1,
    fontWeight: 500,
    fontSize: '1.5rem',
    letterSpacing: '1px',
    fontFamily: 'Roboto, sans-serif',
    color: 'white'
  }));


const Header = () => {
  return (

      <Toolbar>
        <Title >
          Nodes Penalties
        </Title>
      </Toolbar>
  );
};

export default Header;
