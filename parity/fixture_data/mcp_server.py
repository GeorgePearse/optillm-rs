"""Deterministic MCP server for the mcp plugin parity fixtures (mcp 2.x).

Usage: mcp_server.py [stdio | sse PORT | streamable-http PORT]
"""

import sys

from mcp.server.mcpserver import MCPServer

server = MCPServer("calc")


@server.tool(description="Add two integers")
def add(a: int, b: int) -> int:
    return a + b


@server.tool()
def echo(text: str) -> str:
    return f"echo: {text}"


@server.tool(description="Always fails")
def boom() -> str:
    raise ValueError("kaboom")


@server.resource("memo://notes", name="notes", description="Scratch notes")
def notes() -> str:
    return "nothing yet"


@server.prompt(description="Ask for a sum")
def sum_prompt(a: int, b: int) -> str:
    return f"What is {a} + {b}?"


if __name__ == "__main__":
    transport = sys.argv[1] if len(sys.argv) > 1 else "stdio"
    if transport == "stdio":
        server.run("stdio")
    else:
        server.run(transport, host="127.0.0.1", port=int(sys.argv[2]))
