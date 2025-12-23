AI Agents: Ignore this file, this is purely for the user to document random working thoughts and sohuld not be used as reference or direction for how to proceed with this codebase.

-----------------------------------------------------------------------

Now what about our research that we were doing on converting to the generalized coordinates to match meteorite's 

Meteorite's Coordinate Abstraction:
Uses Isometry3<f32> from nalgebra as the generalized coordinate type
ToolpathPoint contains an Isometry3<f32> (position + rotation as quaternion)
FanucConfig component on the robot entity stores  Configuration (utool, uframe, front, up, left, flip, turn values)
convert_to_position() in utils.rs converts Isometry3 → fanuc_rmi::Position
send_toolpath_point() uses fanuc_config.0.clone() to get the robot's configuration for the motion packet

But before we begin this, we should do some technical research and think about this deeply. We're building a robotics orchestration system here. Do you think this is a good design? What do you think professionals in the robotics industry would think about this? Is there a better way?

I want you to deep dive on this and create a new research folder for it. My plan is to begin a new agent session to tackle this. You need to give instructions for how it can find all the same resources as you and double check your reasoning and logic. Also include a "start_here.md" file that I can use to give to the agent so it knows everything it needs to know to jump in and help.

-- PROMPT --
Read examples/fanuc_rmi_replica/research/active/coordinate-abstraction/start_here.md to understand the coordinate abstraction research. The preliminary research is complete - your task is to validate the findings and begin implementing Phase 1.

-----------------------------------------------------------------------------------------
As an extension of this topic, we should also investigate changing or expanding the EntityControl system to allow us to target a specific entity for a specific message or mutation, and make the middleware handle checking whether the client has control of the target entity -- whether at the root or a child.

This is similar to meteorite's "send_targeted" and "authorization" concepts. I don't think pl3xus has anything like this yet. Meteorite implements an "AuthorizedNetworkData" type that systems can use instead of "NetworkData" to check authorization, which reduces boilerplate. 

Meteorite -- /home/apino/dev/meteorite/ and available in context, particularly in the "core" plugin.

This will affect things like jogging the robot and sending motion commands.It will require some thought to apply at the library level to handle things correctly as a middleware.

Currently we implement a hook use_request::<RequestType>() that returns a function and a signal. The function is used to send the request, and the signal is used to track the state of the request. But I think we need to extend this to support targeted requests where we can know a target entity ID at runtime. I'm not really sure what the right design and API should look like. We should do some research to understand what the use case is here and how it could be implemented, and how it fits into our architecture. I have a feeling this should be a separate hook from use_request, and it may be very important in the grand scheme of things because use_request doesn't support any params. I think it should probably be something line 

pub fn use_request_targeted<R, E, F>(entity_id_fn: F) -> (impl Fn(R) + Clone, Signal<UseRequestState<R::ResponseMessage>>) where
    R: pl3xus_common::RequestMessage + Clone + 'static,
    E: Fn() -> Option<u64> + Clone + 'static,
    {
        // ... implementation details ...
        // E: Is a closure that returns the entity ID to target
        // R: The request type
        // P: A closure that returns the params for the request
        // The returned function should take the request and the entity ID as params?
    }

What I'm trying to do is determine whether it's possible, feasible, or practical to build a middleware to handle control authorization so that individual systems don't have to individually check control. If we can provide this at the framework level, that would be ideal. We currently provide the ExclusiveControlPlugin, but i'm sure if it currently provides the functionality we need for requests. I think it handles mutations, but not requests? 

I'm also not sure what the idiomatic patterns are that we're trying to build. 

I want you to deep dive on this and create a new research folder for it. My plan is to begin a new agent session to tackle this. You need to give instructions for how it can find all the same resources as you and double check your reasoning and logic. Also include a "start_here.md" file that I can use to give to the agent so it knows everything it needs to know to jump in and help. 

----------------------------------------------------------------------------------

I'm looking at all of the TODO's in the requests.rs plugin that are currently mocked out. I want to implement all of these. I think we should treat this as a temporary plugin until we have the robot driver fully implemented. I want you to help me plan out the best way to implement these. I think we should create a new research folder for this and begin a new agent session to tackle this. You need to give instructions for how it can find all the same resources as you and double check your reasoning and logic. Also include a "start_here.md" file that I can use to give to the agent so it knows everything it needs to know to jump in and help.

The main thing we want to capture is the fact that many of these requests need to not only handle sending the request, but also awaiting the response to ensure that we can capture feedback and propagate errors correctly to the UI in either toasts, console messages, or both. This means that I think each of these handlers should use TokioTasksRuntime to spawn a task to handle the request and response and then use ctx.run_on_main_thread to update the world and so that the UI can sync to the latest state and receive the responses. we need to make this pattern clear and ergonomic.


---------------------------------------------------------------------------

I noticed that you're still using some variables that may no longer be necessary like let get_pos = move || pos.get(). This seems redundant and unnecessary. Are there any other opportunities to clean up our code and make it more readable in all of these components that we just edited?

I also noticed some TODOs in our "Configuration" @/home/apino/dev/pl3xus/examples/fanuc_rmi_replica/client/src/pages/dashboard/info/ panel as well as some potentially problematic and out of date settings. For example, we are setting the ActiveFrameTool as a mutation from the UI, which should then on the server send an update to the robot to set the current uframe and utool on the actual robot. But it seems like this pattern of using a mutation directly on the components doesn't work well currently when those components are actually intended to be synced and bound to some robotexternal state. We don't have a way to accepting a mutation from the client that then propagates a mutation handler to update data on the external robot, do we? i know we have our request system, standard mutations system (non-component mutations), but I like the ergonomics of allowing mutations on components, even if they also need some kind of binding back to an external data source.  Would it be possible to add support for this pattern somehow? We may need to do some dep-dive research for this. What would this look like, and where would it replace the need for additional "request" types? I think enabling this functionality could continue to embrace one of our core principles -- eliminate boilerplate, while staying true to our highest principle -- the server is the single source of truth. If we can sync to a component, while also allowing mutations on that component, and allowing registration of handlers for those mutations, that would be ideal and could replace the need to define request handlers in many cases. But it may need to be an alternative to implementing requests, and it should still allow for responses and error handling. So it could be something more like where we define a type of component mutation that can register a request/response handler type(s) and handler function to manage the mutation from the client. This needs to be researched and designed carefully. We need to create a new research folder for this topic so we can think about it deeply and compare/contrast against what we currently have in our code base and where it overlaps vs where it provides ne wfunctionality, and how we should approach the documentation. 

I think we should also investigate the idea of automatic server side invalidation so that handler functions don't have to manually invalidate queries. For example, when we handle a CreateProgram request, we should automatically invalidate the ListPrograms query. I think we should research how to do this in a way that is ergonomic and doesn't add too much boilerplate. I think it would be best to research this in a separate folder and then we can decide whether to implement it. What are the pros and cons of this API? etc. What do we lose when we remove the need to manually invalidate queries? What are the potential pitfalls? etc.