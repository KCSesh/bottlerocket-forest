# Setup Phase

You are executing the setup phase of building a kit locally.

## Your Goal

Ensure the local registry is running and configure Infra.toml for the kit.

## Inputs

- Workspace: Read from context_data["workspace"]
- Kit name: Read from `{{workspace}}/input.json` field "kit_name"

## Procedure

1. Start local registry:
   ```bash
   (cd $FOREST_ROOT && brdev registry start)
   ```

2. Navigate to kit directory:
   ```bash
   cd kits/{{kit_name}}
   ```

3. Check if Infra.toml exists. If not, create it:
   ```bash
   if [ ! -f Infra.toml ]; then
     cat > Infra.toml << 'EOF'
[vendor.local]
registry = "localhost:5000"
EOF
   fi
   ```

4. Verify registry is accessible:
   ```bash
   curl -f http://localhost:5000/v2/_catalog
   ```

## Output Format

Write to `{{workspace}}/01-setup.md`:

```markdown
# Setup Complete

## Registry Status
- Running: yes/no
- URL: localhost:5000

## Infra.toml
- Exists: yes/no
- Created: yes/no (if you created it)
- Vendor configured: local

## Kit Directory
- Path: kits/{{kit_name}}
- Verified: yes
```

## Completion

Call `respond_to_leader("success", "<your output>")` when done.
