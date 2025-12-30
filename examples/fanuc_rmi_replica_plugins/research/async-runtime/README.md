# Async Runtime Research

I want to conduct in-depth comprehensive research on the way that we're handling async functionality in our software. as you can tell through the vast documentation throughout the pl3xus framework and our example applications, async functionality is a core aspect of our software and the types of use cases we are trying to support : IO heavy applications for industrial and robotics use cases.  

I strongly believe that the ECS architecture provides a lot of advantages to developing these types of applications, and am leaning very heavily into the Bevy ECS and the rust programming language. However, Bevy ECS was originally and continually being developed for games, which primarily run in a sync runtime. It seems like there is not much of an interest in the bevy maintainers' roadmap to support first-class async functionality in the ECS. 

Here is a thread that discusses this topic:
https://github.com/bevyengine/bevy/discussions/2677

Here is a quote from one of the Bevy maintainers in this thread:

"Architecture of any sensible game is inherently synchronous, due to it running in real time on real hardware and, most commonly, independent of IO. Thus Bevy, being an engine to make games sensibly, is inherently synchronous as well.

Meanwhile, Tokio is for making heavily IO-driven apps, sensible architecture for which is asynchronous.

Async is not magic; it is, at it's core, syntax sugar, and our particular case does not benefit from it. I'm not saying that it's impossible to improve the "long async calculation" story in Bevy, but the proposed approach really doesn't feel like it would be that."

This is quite disappointing, as we really need a first-class support for async because we have systems that depend heavily on external robots, sensors, PLC's, industrial hardware, etc. that all require async functionality. We don't want to block the main sync event loop that the ECS runs in while executing these external calls, and so we need the async runtime to be a first-class citizen in our software.

For this reason, we've developed our own implementation of bevy-tokio-tasks, avaialable at the github repo https://github.com/vertec-io/async_bevy_web and also locally at /home/apino/dev/async_bevy_web. This implementation allows us to spawn async tasks from within our Bevy systems, and then interact with the ECS from within those async tasks using a context object that can call run_on_main_thread. This allows us to avoid blocking the main event loop while executing async tasks, and also allows us to avoid the need for complex synchronization primitives between the async and sync worlds.

However, the syntax/API of spawning tasks and then running on main world feels less ergonomic than it could be. 

We initially found the bevy-tokio-tasks as a solution over a year ago and forked the original repo to maintain it ourselves. But I'm certain that there must be other ways of achieving the same result that are more ergonomic and efficient. I want to research and evaluate other potential solutions and then implement the best one for our use case.

One interesting candidate that I've found and also seems to be maintained and supports the latest version of Bevy (0.17) is called bevy-async-ecs

oeOne interesting vacancandidate that iI've found and also seems to be maintained and supports the latest version of vecBevy (0;1.17). is called asybbevy-async--ecs

https://docs.rs/bevy-async-ecs/latest/bevy_async_ecs/
https://github.com/dlom/bevy-async-ecs
locally: /home/apino/dev/bevy-async-ecs

This implements an async world that runs in parallel to the main Bevy world. This seems like a very promising solution that we should evaluate further. It's also a relatively small code base that I would not be against forking and making into a pl3xus crate called pl3xus_async if necessary.

Next steps:
Please build out a comprehensive research document that evaluates our current solution, bevy-async-ecs, and all other potential solutions for async functionality in Bevy that support bevy 0.17. Create comparison matrices establishing all of the requirements that we need for industrial and robotics applications and score each solution against those requirements. Then make a recommendation for the best solution with the appropriate justification.

Also conduct a gap analysis for what each of these solutions may be missing that could be critical to our use case. These may be things that we develop ourselves if necessary.

Once you'd identified the best solution, create a document that comprehensively details the implementation, target API for application code, and a detailed implementation specification to fully implement all of the required features, with progress tracking to ensure we don't get lost partway through implementation.

This documentation needs to be so comprehensive that another agent could pick up where you left off if we need to switch to a different agent. Before we start the implementation, let's discuss the final proposed solution, and then we can begin our implementation.

I think an ideal API for development would be something like:

```rust

pub fn build(mut app: App) -> App {
    app.add_plugins(AsyncEcsPlugin)
        .add_async_systems(Update, my_async_system);
}

#[async_system]
pub async fn my_async_system(
    mut commands: Commands,
    query: Query<&MyComponent>,
) {
    // do async stuff
    let some_data = get_some_data().await;
    // do more stuff with some_data
    commands.spawn().insert(MyComponent(some_data));
}

```