import { Card, CardContent, Grid, Tooltip, Typography } from "@mui/material";
import { NodeStatus } from "../../../declarations/node_status_canister_backend/node_status_canister_backend.did";
import { FC } from "react";
import { CheckMarkSvg, CrossSvg } from "../utils/svgs";
import * as React from "react";

function trimPrincipal(principal: string): string {
    return principal
        .split('-').slice(0, 1).join('-')
}

type NodeCardsProps = {
    nodesStatus: NodeStatus[]
}

export const NodeCards: FC<NodeCardsProps> = ({ nodesStatus }) => {
    const copyToClipboard = (principal: string) => {
        if (principal == "") {
            return;
        }
        navigator.clipboard.writeText(principal);
    }
    return nodesStatus.map((node) =>
        <Card sx={{ minWidth: 240, maxWidth: 240, margin: 1 }} variant="outlined" key={node.node_id.toString()}>
            <CardContent>
                <Grid container spacing={2}>
                    <Grid item xs={6}>
                        <Typography sx={{ fontSize: 14 }} color="text.secondary" gutterBottom>
                            Node ID:
                        </Typography>
                        <Tooltip title="Copy" placement="bottom">
                            <Typography variant="h6" component="span" onClick={() => copyToClipboard(node.node_id.toString())}
                                sx={{
                                    ":hover": {
                                        cursor: "pointer",
                                        textDecoration: "underline"
                                    }
                                }}>
                                {trimPrincipal(node.node_id.toString())}
                            </Typography>
                        </Tooltip>
                    </Grid>
                    <Grid item xs={6}>
                        <Typography sx={{ fontSize: 14 }} color="text.secondary" gutterBottom>
                            Subnet ID:
                        </Typography>
                        {
                            node.subnet_id.toString() == ""
                                ? <Typography variant="h6" component="div">
                                    N/A
                                </Typography>
                                : <Tooltip title="Copy" placement="bottom">
                                    <Typography variant="h6" component="span" onClick={() => copyToClipboard(node.subnet_id.toString())}
                                        sx={{
                                            ":hover": {
                                                cursor: "pointer",
                                                textDecoration: "underline"
                                            }
                                        }}>
                                        {trimPrincipal(node.subnet_id.toString())}
                                    </Typography>
                                </Tooltip>
                        }
                    </Grid>
                </Grid>
                <Grid container spacing={2} marginTop={1}>
                    <Grid item xs={6} display={"flex"} alignItems={"center"}>
                        <Typography sx={{ fontSize: 20 }} color="text.secondary" gutterBottom>
                            Health:
                        </Typography>
                    </Grid>
                    <Grid item xs={6}>
                        {node.status ? CheckMarkSvg : CrossSvg}
                    </Grid>
                </Grid>
            </CardContent>
        </Card>
    )
}