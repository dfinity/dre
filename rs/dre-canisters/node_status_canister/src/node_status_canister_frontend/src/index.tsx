import * as React from "react";
import { render } from "react-dom";
import { node_status_canister_backend } from "../../declarations/node_status_canister_backend/index.js";
import { NodeStatus } from "../../declarations/node_status_canister_backend/node_status_canister_backend.did.js";
import { Grid, ThemeProvider, createTheme } from "@mui/material";
import CssBaseline from '@mui/material/CssBaseline';
import NavBar from "./components/navBar";
import { SideBar } from "./components/sideBar";
import { BrowserRouter, RouterProvider, createBrowserRouter } from "react-router-dom";
import { NodeCards } from "./components/nodeCards";
import { SearchComp } from "./components/searchComp";
import { NodeTable } from "./components/nodeTable";

const darkTheme = createTheme({
    palette: {
        mode: 'dark',
    },
})

export enum View {
    CARD,
    TABLE
}

const App = () => {
    const [viewOption, setViewOption] = React.useState<View>(View.TABLE);
    const [searchTerm, setSearchTerm] = React.useState<string>("");
    const [nodesStatus, setNodesStatus] = React.useState<NodeStatus[]>([])
    const [nodesStatusDisplay, setNodesStatusDisplay] = React.useState<NodeStatus[]>([])

    function handleTextChange(input: string) {
        setSearchTerm(input);
        applyFilter(nodesStatus);
    }

    function applyFilter(nodes: NodeStatus[]) {
        setNodesStatusDisplay(nodes.filter((node) => node.node_id.toString().includes(searchTerm) || node.subnet_id.toString().includes(searchTerm)));
    }

    React.useEffect(() => {
        function retrieveData() {
            node_status_canister_backend
                .get_node_status()
                .then((nodes) => {
                    setNodesStatus(nodes);
                    applyFilter(nodes);

                })
                .catch((err) => console.error(err));
        }
        retrieveData();
        const interval = setInterval(retrieveData, 10000);

        return () => clearInterval(interval);
    }, [searchTerm]);

    return <ThemeProvider theme={darkTheme}>
        <CssBaseline>
            <NavBar />
            <Grid container spacing={2}>
                <Grid item xs={2}>
                    <SideBar setViewOption={setViewOption}/>
                </Grid>
                <Grid item xs={10} display={"flex"} flexDirection={"row"} flexWrap={"wrap"}>
                    <SearchComp setSearchTerm={handleTextChange} />
                    <Grid display={"flex"} flexDirection={"row"} flexWrap={"wrap"}>
                        {
                            viewOption == View.CARD ? <NodeCards nodesStatus={nodesStatusDisplay} /> : ''
                        }
                        {
                            viewOption == View.TABLE ? <NodeTable nodesStatus={nodesStatusDisplay} /> : ''
                        }
                    </Grid>
                </Grid>
            </Grid>
        </CssBaseline>
    </ThemeProvider>
};

render(<App />, document.getElementById("app"));