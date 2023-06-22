```f-tree
import "nested/impls.tree"
import "nested/impls.tree" {
    grasp => grasp_ball,
}

root place_ball_to_target fallback {
    place_to(
        obj = {"x":1 },
        operation = place([10]),
    )
    retry(5) ask_for_help()
}

sequence place_to(what:object, operation:tree){
    fallback {
        is_approachable(what)
        do_job(approach(what))
    }
    fallback {
         is_graspable(what)
         do_job(approach(what))
    }
    sequence {
         savepoint()
         operation(..)
    }
}

sequence place(where:array){
    is_valid_place(where)
    do_job(slowly_drop({"cord":1}))
}

sequence do_job(action:tree){
    savepoint()
    info_wrapper(action(..))
    savepoint()
}

sequence info_wrapper(action:tree){
    log("before action")
    action(..)
    log("before action")
}

impl log(text:string);

```
 