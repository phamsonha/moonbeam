import { Octokit } from "octokit";
import yargs from "yargs";
import { getCommitAndLabels } from "./github-utils";

async function printInfo(octokit: Octokit, previousVersion: string, nextVersion: string) {
  const owner = "paritytech";
  const prefixes = {
    substrate: "polkadot-",
    polkadot: "release-",
    cumulus: "polkadot-",
  };
  console.log(`# Description\n`);
  console.log(`This ticket is automatically generated using\n`);
  console.log("```");
  console.log(`$ yarn run print-version-bump-info --from ${previousVersion} --to ${nextVersion}`);
  console.log("```");

  const prInfoByLabels = {};
  for (const repo of Object.keys(prefixes)) {
    const previousTag = `${prefixes[repo]}${previousVersion}`;
    const nextTag = `${prefixes[repo]}${nextVersion}`;

    const previousCommit = await octokit.rest.git.getCommit({
      owner,
      repo,
      commit_sha: (
        await octokit.rest.git.getTree({
          owner,
          repo,
          tree_sha: previousTag,
        })
      ).data.sha,
    });
    const nextCommit = await octokit.rest.git.getCommit({
      owner,
      repo,
      commit_sha: (
        await octokit.rest.git.getTree({
          owner,
          repo,
          tree_sha: nextTag,
        })
      ).data.sha,
    });
    console.log(
      `\n## ${repo} (${previousCommit.data.author.date.slice(
        0,
        10
      )} -> ${nextCommit.data.author.date.slice(0, 10)})\n`
    );
    const { commits, prByLabels } = await getCommitAndLabels(
      octokit,
      owner,
      repo,
      previousTag,
      nextTag
    );
    console.log(`https://github.com/${owner}/${repo}/compare/${previousTag}...${nextTag}`);
    console.log("```");
    console.log(`    from: ${previousCommit.data.sha}`);
    console.log(`      to: ${nextCommit.data.sha}`);
    console.log(` commits: ${commits.length}`);
    console.log("```");

    for (const label of Object.keys(prByLabels)) {
      prInfoByLabels[label] = (prInfoByLabels[label] || []).concat(
        prByLabels[label].map((pr) => {
          return `  ${`(${owner}/${repo}#${pr.number}) ${pr.title}`}`;
        })
      );
    }
  }

  console.log(`\n# Important commits by label\n`);
  const excludeRegs = [
    /D5-nicetohaveaudit/,
    /D3-trivia/,
    /D2-notlive/,
    /D1-audited/,
    /C[01234]-/,
    /B0-silent/,
    /A[0-9]-/,
  ];
  for (const labelName of Object.keys(prInfoByLabels).sort().reverse()) {
    if (excludeRegs.some((f) => f.test(labelName))) {
      continue;
    }
    console.log(`\n### ${labelName || "N/A"}\n`);
    for (const prInfo of prInfoByLabels[labelName]) {
      console.log(prInfo);
    }
  }

  console.log(`\n## Review 'substrate-migrations' repo\n`);
  console.log(`https://github.com/apopiak/substrate-migrations#frame-migrations`);
  console.log(`\nThis repository contains a list of FRAME-related migrations which might be`);
  console.log(`relevant to Moonbeam.`);
}

async function main() {
  const argv = yargs(process.argv.slice(2))
    .usage("Usage: npm run print-version-deps [args]")
    .version("1.0.0")
    .options({
      from: {
        type: "string",
        describe: "commit-sha/tag of range start",
      },
      to: {
        type: "string",
        describe: "commit-sha/tag of range end",
      },
    })
    .demandOption(["from", "to"])
    .help().argv;

  const octokit = new Octokit({
    auth: process.env.GITHUB_TOKEN || undefined,
  });

  printInfo(octokit, argv.from, argv.to);
}

main();
