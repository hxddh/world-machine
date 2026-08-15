from pathlib import Path

p = Path("worlds/pocket-universe/src/lib.rs")
text = p.read_text()

old = '''            let growth = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("grow_universe").actor(UNIVERSE),
                )?
                .id;
            let primary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_B,
                &[growth],
            )?;
            let secondary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_E,
                &[primary_outcome],
            )?;
            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_relationship")
                        .caused_by(primary_outcome)
                        .caused_by(secondary_outcome),
                )?
                .id;'''
new = '''            let growth_request = growth_request(&candidate);
            let growth = candidate.execute(&self.actions, &growth_request)?.id;
            let primary_causes = agent_turn_causes(&candidate, SLOT_B, growth);
            let primary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_B,
                &primary_causes,
            )?;
            let secondary_causes = agent_turn_causes(&candidate, SLOT_E, primary_outcome);
            let secondary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_E,
                &secondary_causes,
            )?;
            let relationship_request = with_causes(
                ActionRequest::new("update_relationship")
                    .caused_by(primary_outcome)
                    .caused_by(secondary_outcome),
                relationship_context_causes(&candidate),
            );
            let relationship = candidate
                .execute(&self.actions, &relationship_request)?
                .id;'''
if text.count(old) != 1:
    raise SystemExit(f"manual block count {text.count(old)}")
text = text.replace(old, new, 1)

old = '''            candidate.schedule_at(target, ActionRequest::new("grow_universe").actor(UNIVERSE))?;
            let executed = candidate.advance_to(&self.actions, target)?;
            let growth = executed.last().copied().ok_or_else(|| {
                std::io::Error::other("scheduled Pocket Universe growth did not run")
            })?;
            let primary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_B,
                &[growth],
            )?;
            let secondary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_E,
                &[primary_outcome],
            )?;
            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_relationship")
                        .caused_by(primary_outcome)
                        .caused_by(secondary_outcome),
                )?
                .id;'''
new = '''            let growth_request = growth_request(&candidate);
            candidate.schedule_at(target, growth_request)?;
            let executed = candidate.advance_to(&self.actions, target)?;
            let growth = executed.last().copied().ok_or_else(|| {
                std::io::Error::other("scheduled Pocket Universe growth did not run")
            })?;
            let primary_causes = agent_turn_causes(&candidate, SLOT_B, growth);
            let primary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_B,
                &primary_causes,
            )?;
            let secondary_causes = agent_turn_causes(&candidate, SLOT_E, primary_outcome);
            let secondary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_E,
                &secondary_causes,
            )?;
            let relationship_request = with_causes(
                ActionRequest::new("update_relationship")
                    .caused_by(primary_outcome)
                    .caused_by(secondary_outcome),
                relationship_context_causes(&candidate),
            );
            let relationship = candidate
                .execute(&self.actions, &relationship_request)?
                .id;'''
if text.count(old) != 1:
    raise SystemExit(f"background block count {text.count(old)}")
text = text.replace(old, new, 1)
p.write_text(text)
