#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from pyinfra.operations import apt, server, files, git
from pyinfra.facts.server import LinuxDistribution, Arch
from pyinfra.api import deploy
from pyinfra import host, logger

LEAP_VERSION = '4.0.6'
CDT_VERSION = '4.0.1'
REF_CONTRACTS_COMMIT = '76197b4bc60d8dc91a5d65ecdbf0f785e982e279'


ARCH = host.get_fact(Arch)
if ARCH == 'x86_64':
    ARCH = 'amd64'
DISTRO = host.get_fact(LinuxDistribution)['release_meta']

logger.warning(f"Installing on: {host.get_fact(LinuxDistribution)['release_meta']['PRETTY_NAME']}")


################################################################################
##                                                                            ##
##   Various deploys to install the parts of a running EOS system             ##
##                                                                            ##
################################################################################


@deploy('Install base packages')
def install_base_packages():
    # note: install `libcurl4-gnutls-dev` instead of `libcurl4-openssl-dev` as
    #       the CDT package depends on it
    apt.update()
    apt.packages(['tzdata', 'zip', 'unzip', 'libncurses5', 'wget', 'git',
                  'build-essential', 'cmake', 'curl', 'libboost-all-dev',
                  'libcurl4-gnutls-dev', 'libssl-dev', 'libgmp-dev',
                  'libusb-1.0-0-dev', 'libzstd-dev', 'time', 'pkg-config',
                  'llvm-11-dev', 'nginx', 'jq', 'gdb', 'lldb'])

    # make sure our base folder to install the app exists
    files.directory('/app')

    # upload some utility scripts
    files.put(src='scripts/launch_bg.sh',
              dest='/app/launch_bg.sh',
              mode='755')


@deploy('Install NodeJS and Webpack')
def deploy_nodejs(major_version=18):
    # install recent version of Node
    files.download(src=f'https://deb.nodesource.com/setup_{major_version}.x',
                   dest='/tmp/nodesource_setup.sh',
                   mode='755')
    server.shell(['/tmp/nodesource_setup.sh'])

    apt.packages(['nodejs'])  # TODO: do we need yarnpkg? where do we get it from?
    server.shell(['npm install -D webpack-cli',
                  'npm install -D webpack',
                  'npm install -D webpack-dev-server'],
                  _chdir='/root')


@deploy('Download Leap dev packages and for arm64 arch')
def download_leap_dev(version):
    import httpx

    repo = 'AntelopeIO/experimental-binaries'

    # get Github bearer token for authorization to download packages
    r = httpx.get(f'https://ghcr.io/token?service=registry.docker.io&scope=repository:{repo}:pull').json()
    gh_anon_bearer = r['token']
    headers = {
        'Authorization': f'Bearer {gh_anon_bearer}'
    }

    # get SHA256 digest of the blob with the packages
    url = f'https://ghcr.io/v2/{repo}/manifests/v{version}'
    r = httpx.get(url, headers=headers).raise_for_status().json()
    blob_id = r['layers'][0]['digest']


    url = f'https://ghcr.io/v2/{repo}/blobs/{blob_id}'

    server.shell(commands=[f'curl -s -L -H "Authorization: Bearer {gh_anon_bearer}" '
                           f'https://ghcr.io/v2/{repo}/blobs/"{blob_id}" | tar -xz'],
                 _chdir='/app')

    # r = httpx.get(url, headers=headers, follow_redirects=True).raise_for_status()
    # blob_name = 'leap-packages.tar.gz'
    # with open(blob_name, 'wb') as f:
    #     f.write(r.content)



@deploy('Deploy Antelope Leap')
def deploy_leap(version=None):
    leap_package = f'leap_{version}-{DISTRO["ID"]}{DISTRO["VERSION_ID"]}_{ARCH}.deb'
    leap_url = f'https://github.com/AntelopeIO/leap/releases/download/v{version}/{leap_package}'
    apt.deb(src=leap_url)


@deploy('Deploy Antelope CDT')
def deploy_cdt(version=None):
    cdt_package = f'cdt_{version}_{ARCH}.deb'
    cdt_url = f'https://github.com/AntelopeIO/cdt/releases/download/v{version}/{cdt_package}'
    apt.deb(src=cdt_url)


@deploy('Deploy reference contracts')
def deploy_reference_contracts(commit=None):
    work_dir = '/app/reference_contracts'

    git.repo(src='https://github.com/AntelopeIO/reference-contracts',
             dest=work_dir)
    if commit:
        server.shell(commands=f'git checkout {commit}', _chdir=work_dir)

    build_dir = f'{work_dir}/build'
    files.directory(build_dir)
    server.shell(commands=['cmake ..', 'make -j$(nproc)'],
                 _chdir=build_dir)


@deploy('Create default wallet')
def create_default_wallet():
    privkey = '5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3'

    server.shell(commands=['cleos wallet create --file .wallet.pw',
                           # no need, wallet is already unlocked upon creation
                           # 'cat .wallet.pw | cleos wallet unlock --password',
                           # import main EOSIO account private key
                           f'cleos wallet import --private-key {privkey}'],
                 _chdir='/app')


@deploy('Install reaper script for zombie processes')
def install_reaper_script_for_zombies():
    apt.packages(['runit'])
    # thanks to github.com/phusion
    # this should solve reaping issues of stopped nodes
    files.put(src='scripts/my_init',
              dest='/sbin/my_init',
              mode='755')




################################################################################
##                                                                            ##
##   Execution of the main steps                                              ##
##                                                                            ##
################################################################################

install_base_packages()
#deploy_nodejs(major_version=18)
deploy_leap(version=LEAP_VERSION)
#download_leap_dev(version=LEAP_VERSION)
deploy_cdt(version=CDT_VERSION)
deploy_reference_contracts(commit=REF_CONTRACTS_COMMIT)
create_default_wallet()
install_reaper_script_for_zombies()
