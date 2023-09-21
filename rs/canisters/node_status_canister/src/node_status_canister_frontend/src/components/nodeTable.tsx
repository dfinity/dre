import { FC } from "react"
import { NodeStatus } from "../../../declarations/node_status_canister_backend/node_status_canister_backend.did"
import { Container, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, styled, tableCellClasses } from "@mui/material"
import * as React from "react"
import { CheckMarkSvg, CrossSvg } from "../utils/svgs";

const StyledTableCell = styled(TableCell)(({ theme }) => ({
    [`&.${tableCellClasses.head}`]: {
      backgroundColor: theme.palette.common.black,
      color: theme.palette.common.white,
    },
    [`&.${tableCellClasses.body}`]: {
      fontSize: 14,
    },
  }));
  
  const StyledTableRow = styled(TableRow)(({ theme }) => ({
    '&:nth-of-type(odd)': {
      backgroundColor: theme.palette.action.hover,
    },
  }));

interface NodeTableProps {
    nodesStatus: NodeStatus[]
}


export const NodeTable: FC<NodeTableProps> = ({ nodesStatus }) => {

    return <TableContainer component={Paper} sx={{
        marginLeft: 1
    }}>
    <Table sx={{
        minWidth: '82vw',
        maxWidth: '82vw',
    }} aria-label="customized table" size="small">
      <TableHead>
        <TableRow>
          <StyledTableCell>Node ID</StyledTableCell>
          <StyledTableCell>Subnet ID</StyledTableCell>
          <StyledTableCell>Status</StyledTableCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {nodesStatus.map((node) => (
          <StyledTableRow key={node.node_id.toString()}>
            <StyledTableCell component="th" scope="row">
              {node.node_id.toString()}
            </StyledTableCell>
            <StyledTableCell>{node.subnet_id.toString()}</StyledTableCell>
            <StyledTableCell>{node.status ? CheckMarkSvg : CrossSvg}</StyledTableCell>
          </StyledTableRow>
        ))}
      </TableBody>
    </Table>
  </TableContainer>
} 