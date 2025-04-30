#!/bin/bash

OS=""
FLAG_IS_ARCHIVED_OLD_VERSION=false

lib.debug() {
    echo "[DEBUG] $1" >&2
}

lib.info() {
    echo "[INFO] $1" >&2
}

lib.important() {
    echo "[!! IMPORTANT !!] $1" >&2
}

lib.die() {
    echo "[ERROR] $1" >&2
    exit 1
}

precheck() {
    # check os: results need to be either MacOS or Linux.
    local os_release
    os_release=$(uname -s)
    case $os_release in
        Darwin)
            lib.debug "OS: $os_release"
            OS="macos"
            ;;
        Linux)
            lib.debug "OS: $os_release"
            OS="linux"
            ;;
        *)
            lib.die "Unsupported OS (checked by [uname -s]). Please use MacOS or Linux."
            ;;
    esac

    command -v jq &> /dev/null || lib.die "jq is not installed. Refer to https://command-not-found.com/jq for installation instructions."
}

utils.get_latest_version() {
    # Get latest version from GitHub API, then parsed by jq.
    local version
    version=$(curl -s https://api.github.com/repos/ktlast/pung/releases/latest | jq -r '.tag_name')
    [[ -z $version ]] && lib.die "Failed to get latest version."
    echo "${version}"
}

utils.is_already_installed() {
    local full_name
    full_name=$1
    [[ -d "${full_name}" ]] && return 0
    return 1
}

utils.archive_old_version() {
    local full_name backup_name
    full_name=$1
    backup_name="${full_name}.$(date +%Y%m%d%H%M%S).backup"
    if [[ -d "${full_name}" ]]; then
        lib.debug "Found old version: ${full_name}, archiving it to [${backup_name}]"
        mv "${full_name}" "${backup_name}"
        FLAG_IS_ARCHIVED_OLD_VERSION=true
    fi
}

install () {
    local version
    version=$(utils.get_latest_version)

    case $OS in
        macos)
            lib.debug "Installing Pung for MacOS..."
            full_name="pung-${version}-aarch64-apple-darwin"
            utils.is_already_installed "${full_name}" && utils.archive_old_version "${full_name}"

            # Download the latest release
            download_url="https://github.com/ktlast/pung/releases/download/${version}/${full_name}.tar.gz"
            curl -sL "${download_url}" -o "${full_name}".tar.gz

            # Prepare the directory
            mkdir -p "${full_name}" \
                && tar -xzf "${full_name}".tar.gz -C "${full_name}"


            #############################################################
            # IMPORTANT:
            #   - Users may be block by Mac for security reasons.
            #     Run this command to remove the quarantine attribute.
            #   - Read more in the README.md for more details.
            #############################################################
            lib.important "We are going to remove the quarantine attribute to make Pung executable."
            lib.important "This would be done by running the following command:"
            lib.important "    sudo xattr -d com.apple.quarantine ./pung"
            lib.important "If you are not sure what this means, please press Ctrl+C to exit."
            echo
            read -rp "Press Enter to continue..." </dev/tty
            sudo xattr -d com.apple.quarantine "${full_name}/pung"

            # Make sure it is executable
            [[ -x "${full_name}/pung" ]] || lib.die "Before running Pung, please make sure it is executable. (run 'chmod +x ./pung')"
            ;;
        linux)
            lib.debug "Installing Pung for Linux..."
            full_name="pung-${version}-x86_64-unknown-linux-gnu"
            utils.is_already_installed "${full_name}" && utils.archive_old_version "${full_name}"

            # Download the latest release
            download_url="https://github.com/ktlast/pung/releases/download/${version}/${full_name}.tar.gz"
            curl -sL "${download_url}" -o "${full_name}.tar.gz"

            # Prepare the directory
            mkdir -p "${full_name}" \
                && tar -xzf "${full_name}.tar.gz" -C "${full_name}"

            # Make sure it is executable
            [[ -x "${full_name}/pung" ]] || lib.die "Before running Pung, please make sure it is executable. (run 'chmod +x ./pung')"
            ;;
        *)
            lib.die "Unsupported OS (checked by [uname -s]). Please use MacOS or Linux."
            ;;
    esac
}

main () {
    precheck
    install
    lib.info "Pung is installed successfully."
    lib.info "Run following command to start it:

        cd ${full_name}
        ./pung -u your_name
"
    lib.info "Check '${full_name}/pung --help' for more information."
    if [[ "${FLAG_IS_ARCHIVED_OLD_VERSION}" == "true" ]]; then
        lib.info "Old version is archived to [${full_name}.old.$(date +%Y%m%d%H%M%S).backup]"
    fi
}

main