#!/bin/bash

# Check if tailscale is in PATH
if ! command -v tailscale &> /dev/null; then
    echo "tailscale not found in PATH, setting up distrobox-host-exec wrapper..."
    
    # Create a temporary directory for our wrapper
    WRAPPER_DIR=$(mktemp -d)
    
    # Create the tailscale wrapper script
    cat <<EOF > "$WRAPPER_DIR/tailscale"
#!/bin/bash
distrobox-host-exec tailscale "\$@"
EOF
    
    # Make it executable
    chmod +x "$WRAPPER_DIR/tailscale"
    
    # Add to PATH
    export PATH="$WRAPPER_DIR:$PATH"
    
    # Ensure cleanup on exit
    trap 'rm -rf "$WRAPPER_DIR"' EXIT
fi

# Run the application
cargo run
