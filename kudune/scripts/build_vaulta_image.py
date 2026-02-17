#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from pyinfra.operations import apt, server, files, git
from pyinfra.facts.files import Directory
from pyinfra.facts.server import Arch, Command, LinuxDistribution
from pyinfra.api import deploy
from pyinfra import host, logger
import math

SPRING_VERSION = '1.2.2'
CDT_VERSION = '4.1.1'
SYSTEM_CONTRACTS_VERSION = '3.8.0'
COMPILE_SPRING_CDT = True
NPROC = None

ARCH = host.get_fact(Arch)
if ARCH == 'x86_64':
    ARCH = 'amd64'
elif ARCH == 'aarch64':
    ARCH = 'arm64'
DISTRO = host.get_fact(LinuxDistribution)['release_meta']

logger.warning(f"Installing on: {host.get_fact(LinuxDistribution)['release_meta']['PRETTY_NAME']} (arch: {ARCH})")


################################################################################
##                                                                            ##
##   Various deploys to install the parts of a running Vaulta system          ##
##                                                                            ##
################################################################################


@deploy('Install base packages')
def install_base_packages():
    # note: install `libcurl4-gnutls-dev` instead of `libcurl4-openssl-dev` as
    #       the CDT package depends on it
    apt.update()
    apt.packages(['tzdata', 'zip', 'unzip', 'libncurses5', 'wget', 'git',
                  'build-essential', 'cmake', 'curl',
                  #'libboost-all-dev',  # no need for boost as we have it as a submodule
                  'libcurl4-gnutls-dev', 'libssl-dev', 'libgmp-dev',
                  'libusb-1.0-0-dev', 'libzstd-dev', 'time', 'pkg-config',
                  'llvm-11-dev', 'nginx', 'jq', 'gdb', 'lldb'])

    # make sure our base folder to install the app exists
    files.directory('/app')

    # upload some utility scripts
    files.put(src='scripts/launch_bg.sh',
              dest='/app/launch_bg.sh',
              mode='755')


@deploy('Clone/update git repo')
def git_repo(src, dest, tag=None, branch=None):
    if not host.get_fact(Directory, dest):
        git.repo(src=src, dest=dest, update_submodules=True, recursive_submodules=True)
    else:
        # repo is already checked out, update it
        server.shell('git fetch --all --tags --prune', _chdir=dest)
    if tag or branch:
        if tag:
            commands = [f'git checkout {tag}']
        else:
            commands = [f'git switch {branch}', 'git pull']
        server.shell(commands=commands + ['git submodule update --init --recursive'],
                     _chdir=dest)


SYS_NPROC = 0
SYS_RAM = 0

def get_system_info():
    global SYS_NPROC, SYS_RAM
    cpus = int(host.get_fact(Command, 'nproc'))
    mem = host.get_fact(Command, 'free -k | grep Mem').split()[1]
    mem = int(mem) // (1024*1024)
    SYS_NPROC, SYS_RAM = cpus, mem
    logger.warning(f'Host has {cpus} CPUs and {mem} Gb RAM')


get_system_info()


def nproc(required_gb_per_core=None):
    if NPROC:
        return NPROC
    cpus = SYS_NPROC
    if required_gb_per_core is not None:
        cpus = math.ceil(min(SYS_NPROC, SYS_RAM / required_gb_per_core))
    return cpus


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


@deploy('Compile Antelope Spring')
def compile_spring(tag=None, branch=None):
    work_dir = '/app/spring'
    git_repo(src='https://github.com/AntelopeIO/spring', dest=work_dir, tag=tag, branch=branch)

    build_deps = [
        'build-essential',
        'clang',
        'clang-tidy',
        'cmake',
        'doxygen',
        'git',
        'libxml2-dev',
        'opam', 'ocaml-interp',
        'python3-pip',
        'time',
    ]
    apt.packages(build_deps)

    build_dir = f'{work_dir}/build'
    files.directory(build_dir)
    server.shell(commands=['cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/usr/lib/llvm-11 -DCMAKE_INSTALL_PREFIX=/usr ..',
                           f'make -j{nproc(required_gb_per_core=4)} package',
                           'apt install -y ./antelope-spring_*.deb'],
                 _chdir=build_dir)


@deploy('Deploy Antelope Spring')
def deploy_spring(version=None):
    spring_package = f'antelope-spring_{version}_{ARCH}.deb'
    spring_url = f'https://github.com/AntelopeIO/spring/releases/download/v{version}/{spring_package}'
    apt.deb(src=spring_url)


@deploy('Compile Antelope CDT')
def compile_cdt(tag=None, branch=None):
    work_dir = '/app/cdt'
    git_repo(src='https://github.com/AntelopeIO/cdt', dest=work_dir, tag=tag, branch=branch)

    build_deps = [
        'build-essential',
        'cmake',
        'git',
        # 'libcurl4-openssl-dev', # see beginning of this file, we favor libcurl4-gnutls-dev
        'libgmp-dev',
        'llvm-11-dev',
        'python3-numpy',
        'file',
        'zlib1g-dev',
    ]
    apt.packages(build_deps)

    build_dir = f'{work_dir}/build'
    files.directory(build_dir)
    server.shell(commands=['cmake ..',
                           f'make -j{nproc(required_gb_per_core=2)}',
                           f'cd packages && bash ./generate_package.sh deb ubuntu-22.04 {ARCH}',
                           'apt install -y ./packages/cdt_*.deb'],
                 _chdir=build_dir,
                 _env={'spring_DIR': '/app/spring/build/lib/cmake/spring'})


@deploy('Deploy Antelope CDT')
def deploy_cdt(version=None):
    if version.startswith('4.1'):
        # FIXME: find a better way to do this...
        cdt_package = f'cdt_{version}-1_{ARCH}.deb'
    else:
        cdt_package = f'cdt_{version}_{ARCH}.deb'
    cdt_url = f'https://github.com/AntelopeIO/cdt/releases/download/v{version}/{cdt_package}'
    apt.deb(src=cdt_url)


@deploy('Deploy system contracts')
def deploy_system_contracts(version=None):
    work_dir = '/app/system_contracts'
    tag = f'v{version}' if version else None
    git_repo(src='https://github.com/VaultaFoundation/system-contracts', dest=work_dir, tag=tag)

    build_dir = f'{work_dir}/build'
    files.directory(build_dir)
    server.shell(commands=['cmake ..', f'make -j{nproc()}'],
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
                           # import main Vaulta account private key
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


def cleanup():
    server.shell(commands=['rm -fr /app/spring',
                           'rm -fr /app/cdt'])


################################################################################
##                                                                            ##
##   Execution of the main steps                                              ##
##                                                                            ##
################################################################################

install_base_packages()

#deploy_nodejs(major_version=18)

if COMPILE_SPRING_CDT:
    compile_spring(tag=f'v{SPRING_VERSION}')
    compile_cdt(tag=f'v{CDT_VERSION}')
else:
    deploy_spring(version=SPRING_VERSION)
    deploy_cdt(version=CDT_VERSION)
deploy_system_contracts(version=SYSTEM_CONTRACTS_VERSION)
deploy_fees_system_contract()
create_default_wallet()
install_reaper_script_for_zombies()
cleanup()
