{
  description = "Rust blockchain project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};

        # Multi-instance runner script
        blockchain-runner = pkgs.writeShellScriptBin "blockchain-runner" ''
          #!/usr/bin/env bash

          function usage() {
            echo "Usage: blockchain-runner <name> <api-port> <listen-port> [<peer-ports>]"
            echo "Example: blockchain-runner node1 3001 3000 3002,3004"
            exit 1
          }

          # Parse arguments
          NODE_NAME="$1"
          API_PORT="$2"
          LISTEN_PORT="$3"
          PEER_PORTS="$4"

          # Check required arguments
          if [ -z "$NODE_NAME" ] || [ -z "$API_PORT" ] || [ -z "$LISTEN_PORT" ]; then
            usage
          fi

          # Build connections string
          PEER_ADDRESSES=""
          if [ -n "$PEER_PORTS" ]; then
            PEERS=($(echo $PEER_PORTS | tr ',' ' '))
            for port in "''${PEERS[@]}"; do
              if [ -n "$PEER_ADDRESSES" ]; then
                PEER_ADDRESSES="$PEER_ADDRESSES,"
              fi
              PEER_ADDRESSES="''${PEER_ADDRESSES}127.0.0.1:$port"
            done
          fi

          # Set up environment
          export API_ADDR="127.0.0.1:$API_PORT"
          export LISTEN_ADDR="127.0.0.1:$LISTEN_PORT"
          export DATABASE_PATH="/tmp/ledger-$NODE_NAME"
          export PEER_ADDRESSES="$PEER_ADDRESSES"

          # Echo configuration
          echo "Starting blockchain node: $NODE_NAME"
          echo "API: $API_ADDR"
          echo "Listen: $LISTEN_ADDR"
          echo "Database: $DATABASE_PATH"
          if [ -n "$PEER_ADDRESSES" ]; then
            echo "Connecting to peers: $PEER_ADDRESSES"
          fi

          # Run the blockchain
          exec cargo run
        '';

        # Generate scripts for specific node configurations
        node1 = pkgs.writeShellScriptBin "run-node1" ''
          exec ${blockchain-runner}/bin/blockchain-runner node1 3001 3000
        '';

        node2 = pkgs.writeShellScriptBin "run-node2" ''
          exec ${blockchain-runner}/bin/blockchain-runner node2 3003 3002 3000
        '';

        node3 = pkgs.writeShellScriptBin "run-node3" ''
          exec ${blockchain-runner}/bin/blockchain-runner node3 3005 3004 3000,3002
        '';

        # Generate Zellij layout file
        zellij-layout = pkgs.writeTextFile {
          name = "blockchain-layout.kdl";
          text = ''
            layout {
              pane split_direction="vertical" {
                pane command="bash" {
                  args "-c" "${node1}/bin/run-node1"
                }
                pane split_direction="horizontal" {
                  pane command="bash" {
                    args "-c" "${node2}/bin/run-node2"
                  }
                  pane command="bash" {
                    args "-c" "${node3}/bin/run-node3"
                  }
                }
              }
            }
          '';
        };

        # Zellij launcher script
        zellij-runner = pkgs.writeShellScriptBin "run-blockchain-network" ''
          #!/usr/bin/env bash

          LAYOUT_PATH="${zellij-layout}"

          # Start zellij with the blockchain layout
          exec zellij -l "$LAYOUT_PATH" --session blockchain-network
        '';
      in {
        # Development shell
        devShells.default = pkgs.mkShell {
          packages = [
            blockchain-runner
            node1
            node2
            node3
            zellij-runner
            pkgs.zellij
          ];

          shellHook = ''
            echo "Blockchain development environment ready!"
            echo ""
            echo "Run predefined nodes:"
            echo "  run-node1                - API:3001 Listen:3000"
            echo "  run-node2                - API:3003 Listen:3002 (connects to node1)"
            echo "  run-node3                - API:3005 Listen:3004 (connects to node1 and node2)"
            echo ""
            echo "Run all nodes in zellij:"
            echo "  run-blockchain-network   - Starts all 3 nodes in a zellij session"
            echo ""
            echo "Or run with custom settings:"
            echo "  blockchain-runner <name> <api-port> <listen-port> [<peer-ports>]"
            echo ""
          '';
        };
      }
    );
}
