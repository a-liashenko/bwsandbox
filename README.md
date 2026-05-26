## Bwsandbox

Profile based orchestrator for [bwrap](https://github.com/containers/bubblewrap) and other sandbox utils

### Motivation

The project was born when I was too tired to maintain my bash scripts to keep specific app categories sandboxed. The idea is to keep it a bit smarter than raw bash scripts with some batteries like in-place seccomp compilation.  
Main goals:

- Keep it as explicit as possible
- Allow raw args for services, just like generic bash script
- Minimal magic behind services
- Declarative and re-usable profiles
- Keep it simple, low dependencies, and as a single binary.

I don't like the idea of having a specific profile for each app, so it was designed to create "generic" profiles.  
For example, one profile for games, music, etc. I don't care what they do in sandboxed space, just don't touch my system.  
Another profile for work, just don't touch my system and other projects.  
Trusted profile for apps like Firefox, just in case.

I don't want to add a lot of "default" profiles to avoid bloat like Firejail has. In my opinion, most of the sandboxing tasks can be solved per group of applications, not for every single app.

### Usage

```
bwsandbox [--flags] -- app --arg1 arg2
    -f, --config-file  <path to profile.toml>
    -n, --config-name  <profile name in $XDG_CONFIG_PATH/bwsandbox>
    -a, --config-auto
        Will use <app> as profile name in $XDG_CONFIG_PATH/bwsandbox
```

Example command: `bwsandbox -n generic -- ls -halt`  
App will try to load `$XDG_CONFIG_HOME/bwsandbox/generic.toml` profile and launch `ls -halt` inside bwrap sandbox.  
More info about arguments: [args.rs](./bwsandbox/src/app/args.rs)

### Profile structure

Profiles folder has a [simple profile](./profiles/simple.toml) example to understand how profiles are composed and a [generic profile](./profiles/generic.toml) with more complex configuration. More synthetic examples can be found in the [tests](./bwsandbox/src/tests/profiles) folder.  
A new profile consists of a configuration .toml file and .jinja template for complex argument composition.

```toml
# new_profile.toml
[bwrap]
# Extra args which will be merged with jinja template
inline = [
    { type = "str", value = "--ro-bind" },
    { type = "str", value = "/usr" },
    { type = "str", value = "/usr" },
]

[bwrap.template]
# Jinja template name
name = "new_profile.j2"
# Dir where template and all includes saved
dir = "/path/to/templates"

[bwrap.template.context]
fake_home = { type = "str", value = "/opt/fake_home" }

# [some_service]
# some_service_flag = flag_value
```

### Services

**bwrap** - core of any profile, compose bwrap cli args before launch.  
Extra args added to bwrap:  
`--block-fd` - delay sandboxed app launch before all services initialized  
`--json-status-fd` - track bwrap lifecycle  
`--bind <random_temp_dir>` - temp dir for services to create temp resources (e.g. xdg-dbus-proxy socket)

**seccomp** - compile and export bpf filter  
Extra args added to bwrap:  
`--seccomp <fd>` - pass bpf filter fd to bwrap

**env_mapper** - simple helper to clean and bypass env variables into sandbox  
Extra args added to bwrap:  
`--clearenv` - unset all env variables  
`--setenv` - copy host variable into sandbox  
`--unsetenv` - remove specific variable from sandbox

**dbus** - xdg-dbus-proxy arguments to filter sandbox -> host allowed calls  
Extra args added to bwrap:  
`--symlink` - symlink xdg-dbus-proxy socket from temp dir into sandbox /run dir

**slirp4netns** - host network isolation  
Extra args added to bwrap:
`--unshare-net` - disable host network in sandbox, network will be handled by slirp4netns

**pasta** - host network isolation alternative
Extra args added to bwrap:
`--unshare-net` - disable host network in sandbox, network will be handled by slirp4netns

**appimage** - appimage support  
Extra args added to bwrap:  
`--setenv` - set [APPIMAGE_EXTRACT_AND_RUN](https://github.com/AppImage/AppImageKit/issues/841) to `1`

## Bwsandbox netns helper

> [!WARNING]
> This is a highly experimental tool. It requires `CAP_SYS_ADMIN` which is very dangerous if the binary or dependencies have malicious code or bugs. This helper is worth using only for a very specific scenario.

Main goal is to run bwsandbox inside a pre-created network namespace. F.e. to route the whole application via a VPN living in a separate netns.  
`CAP_SYS_ADMIN` is required to switch into the target net namespace and mount `/etc/netns/ns-name/resolv.conf` into it.  
**ALL** capabilities will be dropped before launching `bwsandbox`.

Usage: `bwsandbox-netns <ns name> <bwsandbox args>`  
Example: `bwsandbox-netns vpn -n generic -- /bin/bash`

## Configuration

Hardcoded config location: `/etc/bwsandbox/netns.toml`  
Config file must be owned by `root:root` and **not** be group or world-writable (max `644`)

```toml
# Path to bwsandbox binary
bwsandbox = "/usr/bin/bwsandbox"
# List of users who allowed to use this tool
allowed_uids = [ 1000 ]
# List of network namespaces allowed to enter
allowed_netns = [ "vpn" ]

```

## Acknowledgments

[bubblejail](https://github.com/igo95862/bubblejail) - for extensive explanation in issue about `--dev` bwrap flag and slirp4netns
