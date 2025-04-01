(the context db is to be written in rust, with access only via a REST api; the file cache and information network are to be written in python)
NOTE: IMPORTANT: the context DB is immutable (or at least copy on write), and it is stored in memory; with regular saves to-disk. also, the context DB accepts context queries - queries where the context tags, and context tag order, and context tag weights are used to filter or select, individual contexts.

(the context DB caches queries and stores them for future use; data can be 'unqiuely' queried by including the original author as the primary context tag)

# (hub-and-spoke expert employee design) (log and tally (and eventquery) analyst freeagent design):
- expose the mcp client to the database via a agent-analyst-expert model local-microservice wrapper, instead of to the editor or direct to db
- when the mcp client is given a question, it ruminates it using the agents as mcp servers, subdividing problem solving;
- every agent is an indipendent mcp client connecting to child mcp clients as mcp servers;
- agents can have different tools to use; if an agent solves a problem ('task sequence complete') or finishes a step of an incremental task that requires validation or iteration, they may alert their parent mcpclient.
    - if the parent approves, they continue to the next step as specified by the parent, or shut down respectively.
    - all steps are logged (to the db), and some logs may trigger other agents
        - (via their vector driven RAG reverse queries event listening documents that compare to the db's new logs' context tags in the db
        - (the event listening documents are used to deduce tags, that are then approximately completed to incoming log tags;
            - to increase accuracy, log's tags (and then log's content if needed / ambiguous) are then reversely used to search these potentially matching listener documents
            - if there is a match, the appropriate analyst agents are notified of the event ).
    - agents are created by their parent mcpclients according to the problems that must be solved or 'missing ingredients';
    agent mcp servers are local but hollow, endpoints using the REST apis and via a 'hub' web_interface 'admin server' to communicate.
    - the 'web interface' recieves all messages as a hub and routes them according to hierachy of the connected mcpclients.
    - all mcp clients can access children and parents via an mcp server that connects to the hub. the collection of mcp clients, servers, and hub, is collectively known as an 'information node';
send the editor 'suggestions' from other users, including AI/mcpClients
clearly mark AI as CPU
the hub logs all ai 'CPU' chat logs to db
The hub can display a specific connected client’s chat, or every client. You can send messages to every client at once or to a single connected client.
Data of the clients is ‘slowly rolled forward’, meaning that before the context window limit is approached, initial context is archived.
