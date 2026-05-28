# Run this configuration only on Cato machines
test "$MACHINE_PROFILE" = cato || return

# Added by OrbStack: command-line tools and integration
# This won't be added again if you remove it.
source ~/.orbstack/shell/init2.fish 2>/dev/null || :

set -gx JJ_DEFAULT_LOG_REVSETS 'trunk() | bookmarks() & trunk().. | mine() & trunk().. & ~(::(bookmarks() | remote_bookmarks()))'

# For `frontend` devcontainers
set -gx DOCKER_HUB_USER hagaiakibayov26971
set -gx DOCKER_HUB_TOKEN dckr_pat_G2ixPMN1zfzqzukGRLOT2a7wIIc
set -gx GIT_USER hagai-akibayov_catonian
set -gx GIT_EMAIL hagai.akibayov@catonetworks.com
set -gx NODE_EXTRA_CA_CERTS $HOME/CatoNetworksTrustedRootCA.pem
set -gx PROXY_HOST cc.dev-us1-m1.catonet.works
