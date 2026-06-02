#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from pyinfra.operations import apt, server, files, git
from pyinfra.facts.files import Directory
from pyinfra.facts.server import Arch, Command, LinuxDistribution
from pyinfra.api import deploy
from pyinfra import host, logger
import math
import re

SPRING_VERSION = '1.2.2'
CDT_VERSION = '4.1.1'
SYSTEM_CONTRACTS_VERSION = '3.10.0'
VAULTA_CONTRACT_VERSION = 'main'
COMPILE_SPRING_CDT = True
NPROC = None
CLEANUP = True

ARCH = host.get_fact(Arch)
if ARCH == 'x86_64':
    ARCH = 'amd64'
elif ARCH == 'aarch64':
    ARCH = 'arm64'
DISTRO = host.get_fact(LinuxDistribution)['release_meta']

SYS_NPROC = 0
SYS_RAM = 0

def get_system_info():
    global SYS_NPROC, SYS_RAM
    cpus = int(host.get_fact(Command, 'nproc'))
    mem = host.get_fact(Command, 'free -k | grep Mem').split()[1]
    mem = int(mem) // (1024*1024)
    SYS_NPROC, SYS_RAM = cpus, mem


get_system_info()


def nproc(required_gb_per_core=None):
    if NPROC:
        return NPROC
    cpus = SYS_NPROC
    if required_gb_per_core is not None:
        cpus = math.ceil(min(SYS_NPROC, SYS_RAM / required_gb_per_core))
    return cpus


spring_cdt_source = 'compiled' if COMPILE_SPRING_CDT else 'from package'

logger.warning(f"Installing on: {DISTRO['PRETTY_NAME']} (arch: {ARCH})")
logger.warning(f'Host has {SYS_NPROC} CPUs and {SYS_RAM} Gb RAM')
logger.warning('Installing the following versions:')
logger.warning(f' - Spring: {SPRING_VERSION} ({spring_cdt_source})')
logger.warning(f' - CDT: {CDT_VERSION} ({spring_cdt_source})')
logger.warning(f' - System contracts: {SYSTEM_CONTRACTS_VERSION} (compiled)')
logger.warning(f' - Vaulta contract: {VAULTA_CONTRACT_VERSION} (compiled)')


################################################################################
##                                                                            ##
##   Various deploys to install the parts of a running Vaulta system          ##
##                                                                            ##
################################################################################

def gitref(git_ref):
    if re.match(r'[0-9]+\.[0-9]+\.[0-9]+', git_ref):
        # tagged versions start with a 'v' prefix
        return f'v{git_ref}'
    return git_ref


@deploy('Install base packages')
def install_base_packages():
    # note: install `libcurl4-gnutls-dev` instead of `libcurl4-openssl-dev` as
    #       the CDT package depends on it
    apt.update()
    apt.packages(['tzdata', 'zip', 'unzip', 'time', 'jq', 'python3',
                  # 'libncurses5', 'libusb-1.0-0-dev', 'libzstd-dev', 'nginx',
                  'wget',  'curl', 'git',
                  'build-essential', 'cmake', 'pkg-config',
                  #'libboost-all-dev',  # no need for boost as we have it as a submodule
                  'llvm-11-dev', 'libcurl4-gnutls-dev', 'libssl-dev', 'libgmp-dev',
                  # 'gdb', 'lldb',
                  ])

    # make sure our base folder to install the app exists
    files.directory('/app')

    # upload some utility scripts
    files.put(src='scripts/launch_bg.sh',
              dest='/app/launch_bg.sh',
              mode='755')


@deploy('Clone/update git repo')
def git_repo(src, dest, git_ref=None):
    if not host.get_fact(Directory, dest):
        git.repo(src=src, dest=dest, update_submodules=True, recursive_submodules=True)
    else:
        # repo is already checked out, update it
        server.shell('git fetch --all --tags --prune', _chdir=dest)

    if git_ref:
        git_ref = gitref(git_ref)
        commands = [f'git checkout {git_ref}']
    else:
        commands = []

    commands += [
        # only do a `git pull` if we are on a branch (ie: not detached)
        '([ $(git rev-parse --abbrev-ref --symbolic-full-name HEAD) != "HEAD" ] && git pull) || true',
        'git submodule update --init --recursive',
    ]
    server.shell(commands=commands, _chdir=dest)


@deploy('Compile Antelope Spring')
def compile_spring(git_ref=None):
    work_dir = '/app/spring'
    git_repo(src='https://github.com/AntelopeIO/spring', dest=work_dir, git_ref=git_ref)

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
                           f'make -j{nproc(required_gb_per_core=4)}',
                           # for some reason, parallel build of the package fails in amd64 images on Apple silicon
                           # with bad file descriptors errors. Couldn't pinpoint it exactly, but `-j1` solves it
                           'make -j1 package',
                           'apt install -y ./antelope-spring_*.deb'],
                 _chdir=build_dir)


@deploy('Deploy Antelope Spring')
def deploy_spring(version=None):
    spring_package = f'antelope-spring_{version}_{ARCH}.deb'
    spring_url = f'https://github.com/AntelopeIO/spring/releases/download/v{version}/{spring_package}'
    apt.deb(src=spring_url)


@deploy('Compile Antelope CDT')
def compile_cdt(git_ref=None):
    work_dir = '/app/cdt'
    git_repo(src='https://github.com/AntelopeIO/cdt', dest=work_dir, git_ref=git_ref)

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
    server.shell(commands=['cmake -DCMAKE_BUILD_TYPE=Release ..',
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
    git_repo(src='https://github.com/VaultaFoundation/system-contracts', dest=work_dir, git_ref=version)

    build_dir = f'{work_dir}/build'
    files.directory(build_dir)
    server.shell(commands=['cmake -DCMAKE_BUILD_TYPE=Release ..', f'make -j{nproc()}'],
                 _chdir=build_dir)


@deploy('Deploy fees system contract')
def deploy_fees_system_contract(version=None):
    work_dir = '/app/eosio.fees'

    git.repo(src='https://github.com/VaultaFoundation/eosio.fees',
             dest=work_dir)
    if version:
        server.shell(commands=f'git checkout {gitref(version)}', _chdir=work_dir)

    server.shell(commands=['cdt-cpp eosio.fees.cpp -I ./include'],
                 _chdir=work_dir)


@deploy('Deploy Vaulta system contract')
def deploy_vaulta_contract():
    work_dir = '/app/vaulta_system_contract'
    git_repo(src='https://github.com/VaultaFoundation/vaulta-system-contract', dest=work_dir, git_ref='main')

    build_dir = f'{work_dir}/build'
    files.directory(build_dir)
    server.shell(commands=['cmake -DCMAKE_BUILD_TYPE=Release ..', f'make -j{nproc()}'],
                 _env={'SYSTEM_CONTRACTS_PATH': '/app/system-contracts/build/contracts'},
                 _chdir=build_dir)


@deploy('Create default wallet')
def create_default_wallet():
    privkey = '5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3'  # eosio dev key

    server.shell(commands=['cleos wallet create --file .wallet.pw || true',
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
                           'rm -fr /app/cdt',
                           'rm -fr /tmp/pyinfra-*'])

    # remove build dependencies
    apt.packages(['libgmp-dev', 'llvm-11-dev'], present=False)
    server.shell(commands=['apt-get -y autoremove',
                           'apt-get clean'])



################################################################################
##                                                                            ##
##   Execution of the main steps                                              ##
##                                                                            ##
################################################################################

install_base_packages()

if COMPILE_SPRING_CDT:
    compile_spring(git_ref=SPRING_VERSION)
    compile_cdt(git_ref=CDT_VERSION)
else:
    deploy_spring(version=SPRING_VERSION)
    deploy_cdt(version=CDT_VERSION)

deploy_system_contracts(version=SYSTEM_CONTRACTS_VERSION)
deploy_fees_system_contract()
deploy_vaulta_contract()
create_default_wallet()
install_reaper_script_for_zombies()

if CLEANUP:
    cleanup()
