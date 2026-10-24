// 将大量 Node 小文件合并为单个归档，减少 NSIS 打包和释放时的逐文件开销。
import { spawnSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs'
import { join, resolve, sep } from 'node:path'
import process from 'node:process'

const root = resolve(import.meta.dirname, '..')
const bundle = join(root, 'src-tauri/resources/bundle')
const dsh = join(bundle, 'dsh')
const archive = join(bundle, 'dsh.tar')
const temporary = join(bundle, `.dsh-${process.pid}.tar`)
const staging = join(root, '.tmp', `dsh-pack-${process.pid}`)

interface JunctionLink { path: string, target: string }

function run(args: string[]) {
  const result = spawnSync('tar.exe', args, { stdio: 'inherit', windowsHide: true })
  if (result.error || result.status !== 0)
    throw new Error(`tar.exe failed: ${result.error || result.status}`)
}

function relativePath(path: string) {
  return path.slice(dsh.length + 1).split(sep).join('/')
}

function findDuplicateZod() {
  const packages = join(dsh, 'node_modules/@deepseek-ai')
  const groups = new Map<string, string[]>()
  for (const entry of readdirSync(packages, { withFileTypes: true })) {
    if (!entry.isDirectory())
      continue
    const candidate = join(packages, entry.name, 'node_modules/zod')
    const manifest = join(candidate, 'package.json')
    if (!existsSync(manifest))
      continue
    const version = JSON.parse(readFileSync(manifest, 'utf8')).version as string
    const paths = groups.get(version) || []
    paths.push(candidate)
    groups.set(version, paths)
  }
  return [...groups.values()].filter(paths => paths.length > 1)
}

if (!existsSync(join(dsh, 'node_modules/@deepseek-ai/dsh/lib/bin.js')))
  throw new Error('DSH runtime is missing; run prepare-runtime first')

rmSync(temporary, { force: true })
rmSync(staging, { recursive: true, force: true })
try {
  const duplicateGroups = findDuplicateZod()
  const links: JunctionLink[] = []
  const shared: string[] = []
  const excludes: string[] = []
  const stagingDsh = join(staging, 'dsh')
  mkdirSync(stagingDsh, { recursive: true })

  for (const paths of duplicateGroups) {
    const version = JSON.parse(readFileSync(join(paths[0], 'package.json'), 'utf8')).version as string
    const target = `shared/zod-${version}`
    cpSync(paths[0], join(stagingDsh, target), { recursive: true })
    shared.push(`dsh/${target}`)
    for (const path of paths) {
      const link = relativePath(path)
      links.push({ path: link, target })
      excludes.push(`--exclude=dsh/${link}`)
    }
  }

  console.log('Packing DSH runtime into a single archive')
  // 不在此处压缩：NSIS 的 solid LZMA 会对整个归档统一压缩，避免重复工作。
  run(['-cf', temporary, ...excludes, '-C', bundle, 'dsh'])

  if (links.length) {
    writeFileSync(join(stagingDsh, 'runtime-junctions.json'), JSON.stringify({ links }, null, 2))
    cpSync(join(root, 'scripts/runtime/install-junctions.ps1'), join(stagingDsh, 'install-junctions.ps1'))
    run(['-rf', temporary, '-C', staging, ...shared, 'dsh/runtime-junctions.json', 'dsh/install-junctions.ps1'])
    console.log(`Deduplicated ${links.length} nested Zod installations`)
  }

  rmSync(archive, { force: true })
  renameSync(temporary, archive)
  console.log('DSH runtime archive prepared')
}
finally {
  rmSync(temporary, { force: true })
  rmSync(staging, { recursive: true, force: true })
}
