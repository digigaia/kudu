#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from pyinfra.operations import apt, server, files, git
from pyinfra.facts.server import LinuxDistribution, Arch
from pyinfra.api import deploy
from pyinfra import host, logger

SPRING_VERSION = '1.1.5'
CDT_VERSION = '4.1.0'
SYSTEM_CONTRACTS_VERSION = '3.8.0'


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


@deploy('Deploy Antelope Spring')
def deploy_spring(version=None):
    spring_package = f'antelope-spring_{version}_{ARCH}.deb'
    spring_url = f'https://github.com/AntelopeIO/spring/releases/download/v{version}/{spring_package}'
    apt.deb(src=spring_url)


@deploy('Deploy Antelope CDT')
def deploy_cdt(version=None):
    if version == '4.1.0':
        # FIXME: find a better way to do this...
        cdt_package = f'cdt_{version}-1_{ARCH}.deb'
    else:
        cdt_package = f'cdt_{version}_{ARCH}.deb'
    cdt_url = f'https://github.com/AntelopeIO/cdt/releases/download/v{version}/{cdt_package}'
    apt.deb(src=cdt_url)


@deploy('Deploy system contracts')
def deploy_system_contracts(version=None):
    work_dir = '/app/system_contracts'

    git.repo(src='https://github.com/VaultaFoundation/system-contracts',
             dest=work_dir)
    if version:
        server.shell(commands=f'git checkout v{version}', _chdir=work_dir)

    build_dir = f'{work_dir}/build'
    files.directory(build_dir)
    server.shell(commands=['cmake ..', 'make -j$(nproc)'],
                 _chdir=build_dir)


@deploy('Deploy fees system contract')
def deploy_fees_system_contract(version=None):
    work_dir = '/app/eosio.fees'

    git.repo(src='https://github.com/VaultaFoundation/eosio.fees',
             dest=work_dir)
    if version:
        server.shell(commands=f'git checkout v{version}', _chdir=work_dir)

    server.shell(commands=['cdt-cpp eosio.fees.cpp -I ./include'],
                 _chdir=work_dir)


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
deploy_spring(version=SPRING_VERSION)
deploy_cdt(version=CDT_VERSION)
deploy_system_contracts(version=SYSTEM_CONTRACTS_VERSION)
deploy_fees_system_contract()
create_default_wallet()
install_reaper_script_for_zombies()
