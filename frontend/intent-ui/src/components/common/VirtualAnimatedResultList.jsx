import { Motion } from "solid-motionone";
import VirtualAnimatedList from "./VirtualAnimatedList";

function VirtualAnimatedResultList(props) {
  const items = props.items;
  const pinnedOffset = Number(props.pinnedOffset || 0);
  return (
    <VirtualAnimatedList
      items={items}
      estimateSize={Number(props.estimateSize || 120)}
      overscan={Number(props.overscan || 2)}
      renderItem={(result, index) => (
        <Motion.div
          initial={{ opacity: 0, y: 10, scale: 0.985 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 0.26, delay: (pinnedOffset + index) * 0.14, easing: [0.22, 1, 0.36, 1] }}
          class="relative group"
        >
          {props.renderItem?.(result, index)}
        </Motion.div>
      )}
    />
  );
}

export default VirtualAnimatedResultList;
