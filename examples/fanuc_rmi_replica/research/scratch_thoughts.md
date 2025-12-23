
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

-----------------------------------------------------------------------------------------


I'm looking at all of the TODO's in the requests.rs plugin that are currently mocked out. I want to implement all of these. I think we should treat this as a temporary plugin until we have the robot driver fully implemented. I want you to help me plan out the best way to implement these. I think we should create a new research folder for this and begin a new agent session to tackle this. You need to give instructions for how it can find all the same resources as you and double check your reasoning and logic. Also include a "start_here.md" file that I can use to give to the agent so it knows everything it needs to know to jump in and help.

The main thing we want to capture is the fact that many of these requests need to not only handle sending the request, but also awaiting the response to ensure that we can capture feedback and propagate errors correctly to the UI in either toasts, console messages, or both. This means that I think each of these handlers should use TokioTasksRuntime to spawn a task to handle the request and response and then use ctx.run_on_main_thread to update the world and so that the UI can sync to the latest state and receive the responses. we need to make this pattern clear and ergonomic.

-----------------------------------------------------------------------------------------
We've expanded the use_sync_.... hooks to include use_sync_entity_component and use_sync_entity_component_store. I feel like these names are getting unnecessarily long and may be confusing. I think we should consider renaming these to use_entity_component and use_entity_component_store. What do you think? We may also want to consider expanding the patterns we've developed with these two hooks to any of the other hooks that take an entity ID as a param (if they exist). It would be good to to make sure our hook naming conventions are consistent and intuitive and the framework provides clarity on the idiomatic patterns and anti-patterns.

I want you to deep dive on this and create a new research folder for it. My plan is to begin a new agent session to tackle this. You need to give instructions for how it can find all the same resources as you and double check your reasoning and logic. Also include a "start_here.md" file that I can use to give to the agent so it knows everything it needs to know to jump in and help.

-----------------------------------------
Go ahead and make a commit here ato capture our conversation so far.

I guess this brings us back to the discussion around the request/response pattern. The program commands use the request/response pattern, not targeted messages, but shouldn't these request patterns also support at minimum targeted parameters specifying which entity the request is for, if it's applicable? For example, the StartProgram, PauseProgram, ResumeProgram, StopProgram, UnloadProgram requests should all target the ActiveSystem entity. I guess the fundamental question here is, should we require these requests to be processed through the middleware first, and register some policy including a targeted entity or should we include it as part of the request params? 

Ultimately there needs to be flexibiltiy for the user to decide so we should probably support the same variations that we support for messages, but we should be clear about the different ways to handle these. What would be idiomatic, ergonomic, and conventional for engineers looking at this framework to look at our API and think immediately "yes, this makes sense and how it should be done."

In this case with setting the SpeedOverride, I believe it should be a request because we need to ensure the response is received and errors are handled correctly. Which is starting to make me think that all of our Messages should really be Requests in this implementation. What do you think? I'm trying to figure out where the message and targeted message makes more sense than using requests with the ability to have responses. 

I want you to deep dive on this and create a new research folder for it and think about it deeply. I want to make sure that we have an excellent API that follows proper conventions and patterns that are immediately intuitive to engineers.

Once we have a solution for this, then we can decide whether these other commands, including the ConnectToRobot message should be converted to requests (or targeted requests?) as well instead of a targeted message.

---------------------------------------------------------------------------

I noticed that you're still using some variables that may no longer be necessary like let get_pos = move || pos.get(). This seems redundant and unnecessary. Are there any other opportunities to clean up our code and make it more readable in all of these components that we just edited?

I also noticed some TODOs in our "Configuration" @/home/apino/dev/pl3xus/examples/fanuc_rmi_replica/client/src/pages/dashboard/info/ panel as well as some potentially problematic and out of date settings. For example, we are setting the ActiveFrameTool as a mutation from the UI, which should then on the server send an update to the robot to set the current uframe and utool on the actual robot. But it seems like this pattern of using a utation directly on the omponents doesn't work well currently when those components are actually intended to be synced and bound to some robotexternal state. We don't have a way to accepting a mutation from the client that then propagates a mutation handler to update data on the external robot, do we? i know we have our request system, but I like the ergonomics of allowing mutations on components, even if they're not purely server state, but also need some kind of binding back to an external data soure. I feel like we had this type of functinoality at some point in the past, but maybe I'm missing something. Would it be possible to add suppotr for this pattern somehow? We may need to do some dep-dive research for this. 