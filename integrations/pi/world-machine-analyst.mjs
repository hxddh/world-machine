import { AnalystJsonlClient, providerSafeToolName } from "./world-machine-analyst-client.mjs";

const PROGRAM_ENV = "WORLD_MACHINE_ANALYST_PROGRAM";
const LEFT_ENV = "WORLD_MACHINE_LEFT_ARCHIVE";
const RIGHT_ENV = "WORLD_MACHINE_RIGHT_ARCHIVE";

export default function worldMachineAnalyst(pi) {
  let client = null;
  let activeToolNames = [];
  const registeredHostTools = new Map();

  const onProcessExit = () => client?.kill();
  process.on("exit", onProcessExit);

  pi.on("session_start", async () => {
    pi.setActiveTools([]);
    try {
      const descriptors = await startAndReadCatalog();
      registerCatalog(descriptors);
      activeToolNames = descriptors.map((descriptor) => providerSafeToolName(descriptor.name));
      pi.setActiveTools(activeToolNames);
    } catch (error) {
      await closeClient();
      throw error;
    }
  });

  pi.on("session_shutdown", async () => {
    pi.setActiveTools([]);
    activeToolNames = [];
    await closeClient();
    process.off("exit", onProcessExit);
  });

  async function startAndReadCatalog() {
    await closeClient();
    const program = requiredEnv(PROGRAM_ENV);
    const leftArchive = requiredEnv(LEFT_ENV);
    const rightArchive = requiredEnv(RIGHT_ENV);
    client = AnalystJsonlClient.spawn(program, [leftArchive, rightArchive]);
    const descriptors = await client.listTools();
    if (descriptors.length === 0) {
      throw new Error("World Machine analyst host exposed an empty tool catalog");
    }
    const names = new Set();
    for (const descriptor of descriptors) {
      validateDescriptor(descriptor);
      const providerName = providerSafeToolName(descriptor.name);
      if (names.has(providerName)) {
        throw new Error(`World Machine analyst tool name collision after provider normalization: ${providerName}`);
      }
      names.add(providerName);
    }
    return descriptors;
  }

  function registerCatalog(descriptors) {
    for (const descriptor of descriptors) {
      const providerName = providerSafeToolName(descriptor.name);
      const previousHostName = registeredHostTools.get(providerName);
      if (previousHostName && previousHostName !== descriptor.name) {
        throw new Error(`World Machine analyst tool mapping changed for ${providerName}`);
      }
      if (previousHostName) continue;

      registeredHostTools.set(providerName, descriptor.name);
      pi.registerTool({
        name: providerName,
        label: descriptor.name,
        description: descriptor.description,
        promptSnippet: `${descriptor.name}: read-only World evidence analysis`,
        promptGuidelines: [
          `Use ${providerName} only for read-only analysis of the World archives bound to this analyst session.`,
          "Do not infer that analyst tools can mutate the World or choose different archive paths.",
        ],
        parameters: descriptor.input_schema,
        executionMode: "sequential",
        async execute(toolCallId, params, signal) {
          if (!client) {
            throw new Error("World Machine analyst session is not active");
          }
          try {
            const output = await client.invoke(toolCallId, descriptor.name, params, signal);
            return {
              content: [{ type: "text", text: JSON.stringify(output) }],
              details: {
                worldMachineTool: descriptor.name,
                output,
              },
            };
          } catch (error) {
            if (signal?.aborted) {
              await closeClient();
            }
            throw error;
          }
        },
      });
    }
  }

  async function closeClient() {
    const current = client;
    client = null;
    if (current) await current.shutdown();
  }
}

function requiredEnv(name) {
  const value = process.env[name];
  if (!value) {
    throw new Error(`World Machine Pi analyst requires ${name}`);
  }
  return value;
}

function validateDescriptor(descriptor) {
  if (!descriptor || typeof descriptor !== "object") {
    throw new Error("World Machine analyst catalog contains a non-object descriptor");
  }
  if (typeof descriptor.name !== "string" || descriptor.name.length === 0) {
    throw new Error("World Machine analyst catalog contains a tool without a name");
  }
  if (typeof descriptor.description !== "string") {
    throw new Error(`World Machine analyst tool ${descriptor.name} is missing a description`);
  }
  if (descriptor.read_only !== true) {
    throw new Error(`World Machine Pi analyst refuses non-read-only tool ${descriptor.name}`);
  }
  if (!descriptor.input_schema || typeof descriptor.input_schema !== "object" || Array.isArray(descriptor.input_schema)) {
    throw new Error(`World Machine analyst tool ${descriptor.name} has an invalid input schema`);
  }
}
